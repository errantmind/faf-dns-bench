/*
FaF is a high performance dns benchmarking tool
Copyright (C) 2023  James Bates

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

// Domains fetched from Alexa on 2023-02-09 from: s3.amazonaws.com/alexa-static/top-1m.csv.zip

#![allow(clippy::missing_safety_doc, clippy::uninit_assumed_init)]
#![feature(let_chains)]

mod args;
mod const_sys;
mod data;
mod statics;
mod time;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[repr(C)]
pub union epoll_data {
   pub ptr: isize,
   pub fd: i32,
   pub uint32_t: u32,
   pub uint64_t: u64,
}

#[repr(C, packed)]
pub struct epoll_event {
   pub events: u32,
   pub data: epoll_data,
}

fn main() {
   print_version();
   assert!(statics::DOMAINS_TO_INCLUDE <= u16::MAX as usize, "You can not include more than 65535 (u16 MAX) domains");

   let domains = data::read_domains(statics::DOMAINS_TO_INCLUDE);
   assert!(domains.len() == statics::DOMAINS_TO_INCLUDE);

   let mut queries = Vec::with_capacity(statics::DOMAINS_TO_INCLUDE);
   let mut query_buf: Vec<u8> = vec![0; 512];

   // Pre-compute the DNS query for each domain
   for (id, domain) in domains.iter().enumerate() {
      queries.push(construct_query(domain.as_str(), id as u16, &mut query_buf).to_vec());

      // Don't need to zero the buffer, it is entirely overwritten
      //unsafe { query.as_mut_ptr().write_bytes(0, 512) };
   }

   let (dns_server, dns_port) = if let Some(server) = statics::ARGS.server.as_ref() {
      (server.to_string(), statics::ARGS.port)
   } else {
      let nslookup_output = std::process::Command::new("nslookup").arg(".").output().expect("nslookup either doesn't exist or failed to start.\nEither install nslookup or this programs options to manually specify a server/port");
      let r: regex::Regex = regex::Regex::new(r"([0-9]{1,3}.[0-9]{1,3}.[0-9]{1,3}.[0-9]{1,3}#\d+)").unwrap();
      let server_and_port = r.captures(std::str::from_utf8(&nslookup_output.stdout).unwrap()).unwrap().get(0).unwrap().as_str().to_string();
      let server_and_port_split = server_and_port.split('#').collect::<Vec<&str>>();
      let server = server_and_port_split[0];
      let port = server_and_port_split[1].parse::<u16>().unwrap();
      (server.to_string(), port)
   };

   println!("Benchmarking {dns_server}#{dns_port}\n");

   let mut local_udp_socket = mio::net::UdpSocket::bind("0.0.0.0:59233".parse().unwrap()).unwrap();

   let dns_addr = format!("{}:{}", dns_server.as_str(), dns_port);
   local_udp_socket.connect(dns_addr.parse().unwrap()).unwrap();

   let mut poll = mio::Poll::new().unwrap();
   const MIO_TOKEN: mio::Token = mio::Token(222);
   poll.registry().register(&mut local_udp_socket, MIO_TOKEN, mio::Interest::READABLE).unwrap();
   let mut events = mio::Events::with_capacity(statics::MAX_CONCURRENCY + 1);

   let mut current_time = time::get_timespec();
   let mut timings_start: [time::timespec; statics::DOMAINS_TO_INCLUDE] = unsafe { core::mem::zeroed() };
   let mut timings_end: [time::timespec; statics::DOMAINS_TO_INCLUDE] = unsafe { core::mem::zeroed() };
   let mut current_query = 0;
   let mut completed_queries: usize = 0;
   let mut outstanding_queries: usize = 0;
   let mut finished_querying = false;
   let mut response_buf: Vec<u8> = vec![0; 512];

   loop {
      if completed_queries == statics::DOMAINS_TO_INCLUDE
         || time::get_elapsed_ms(&time::get_timespec(), &current_time) > statics::COLLECTION_TIMEOUT_MS
      {
         break;
      }

      if outstanding_queries < statics::MAX_CONCURRENCY && !finished_querying {
         for _ in 0..statics::MAX_CONCURRENCY - outstanding_queries {
            if current_query >= statics::DOMAINS_TO_INCLUDE {
               finished_querying = true;
               break;
            }

            timings_start[current_query] = time::get_timespec();
            local_udp_socket.send(&queries[current_query]).unwrap();
            current_query += 1;
            outstanding_queries += 1;
         }
      }
      let _ = poll.poll(&mut events, Some(std::time::Duration::from_millis(1000)));

      for event in events.iter() {
         if event.token() == MIO_TOKEN {
            let num_bytes_read_maybe = local_udp_socket.recv(response_buf.as_mut_slice());
            if let Ok(num_bytes_read) = num_bytes_read_maybe && num_bytes_read > 0 {

               let id_from_response = u16::swap_bytes(unsafe { *(response_buf.as_ptr() as *const u16) });
               timings_end[id_from_response as usize] = time::get_timespec();
               let domain_str = get_question_as_string(response_buf.as_ptr(), num_bytes_read);
               if statics::ARGS.debug {
                  println!("{num_bytes_read:>4}b -> {domain_str}");
                  let response_slice = &response_buf[0..num_bytes_read];
                  println!("{response_slice:?}");
               }

               outstanding_queries -= 1;
               completed_queries += 1;
               current_time = time::get_timespec();
            }
         }
      }
   }

   {
      use statrs::statistics::Statistics;

      // Benchmark done, calculate stats
      let mut elapsed_ns = Vec::with_capacity(statics::DOMAINS_TO_INCLUDE);

      for i in 0..statics::DOMAINS_TO_INCLUDE {
         if (timings_start[i].tv_sec == 0 && timings_start[i].tv_nsec == 0) || (timings_end[i].tv_sec == 0 && timings_end[i].tv_nsec == 0) {
            continue;
         } else {
            elapsed_ns.push(time::get_elapsed_ns(&timings_end[i], &timings_start[i]));
         }
      }

      elapsed_ns.sort_unstable();

      let elapsed_ns_as_f64: Vec<f64> = elapsed_ns.iter().map(|x| *x as f64).collect();
      let median = elapsed_ns[elapsed_ns.len() / 2] as f64;
      let mean = Statistics::mean(&elapsed_ns_as_f64);
      let std_dev = Statistics::std_dev(&elapsed_ns_as_f64);
      let min = elapsed_ns[0] as f64;
      let max = elapsed_ns[elapsed_ns.len() - 1] as f64;

      if elapsed_ns_as_f64.len() != statics::DOMAINS_TO_INCLUDE {
         println!(
            "WARN: DNS resolution failed for {} domains which will be ignored.\n",
            statics::DOMAINS_TO_INCLUDE - elapsed_ns_as_f64.len()
         );
      }

      println!("results.json");
      println!("{{");
      println!("  {:<13} : {:>7.2},", "\"median_ms\"", median / 1_000_000f64);
      println!("  {:<13} : {:>7.2},", "\"mean_ms\"", mean / 1_000_000f64);
      println!("  {:<13} : {:>7.2},", "\"stddev_ms\"", std_dev / 1_000_000f64);
      println!("  {:<13} : {:>7.2},", "\"min_ms\"", min / 1_000_000f64);
      println!("  {:<13} : {:>7.2},", "\"max_ms\"", max / 1_000_000f64);
      println!("  {:<13} : {:>7.2},", "\"n_samples\"", statics::DOMAINS_TO_INCLUDE);
      println!("  {:<13} : {:>7},", "\"concurrency\"", statics::MAX_CONCURRENCY);
      println!("  {:<13} : \"{:>7}#{}\"", "\"server\"", dns_server, dns_port);

      println!("}}");
   }
}

fn print_version() {
   println!("{} v{} | repo: https://github.com/errantmind/faf-dns-bench\n", statics::PROJECT_NAME, statics::VERSION,);
}

fn construct_query<'a>(domain: &str, id: u16, dest_buf: &mut [u8]) -> &'a [u8] {
   assert!(dest_buf.len() == 512);

   unsafe {
      let ptr_start = dest_buf.as_ptr() as *const u8;
      let mut ptr_walker = dest_buf.as_ptr() as *mut u8;
      *(ptr_walker as *mut u16) = u16::swap_bytes(id);
      ptr_walker = ptr_walker.add(2);
      *(ptr_walker as *mut u16) = u16::swap_bytes(256); // Set config bytes, only need to set recursion to 1, everything else to 0
      ptr_walker = ptr_walker.add(2);
      *(ptr_walker as *mut u16) = u16::swap_bytes(1); // Set QDCOUNT to 1
      ptr_walker = ptr_walker.add(2);
      ptr_walker = ptr_walker.add(6); // Skip ANCOUNT, NSCOUNT, ARCOUNT

      // Set QNAME
      let first_domain_split: Vec<&str> = domain.split('.').collect();
      for segment in first_domain_split.into_iter() {
         let segment_len = segment.len();
         *ptr_walker = segment_len as u8;
         ptr_walker = ptr_walker.add(1);
         std::ptr::copy_nonoverlapping(segment.as_ptr(), ptr_walker, segment_len);
         ptr_walker = ptr_walker.add(segment_len);
      }
      *ptr_walker = 0; // Set null terminator for QNAME
      ptr_walker = ptr_walker.add(1);
      *(ptr_walker as *mut u16) = u16::swap_bytes(1); // Set QTYPE
      ptr_walker = ptr_walker.add(2);
      *(ptr_walker as *mut u16) = u16::swap_bytes(1); // Set QCLASS
      ptr_walker = ptr_walker.add(2);

      core::slice::from_raw_parts(ptr_start, ptr_walker as usize - ptr_start as usize)
   }
}

#[inline]
fn get_question_as_string(dns_buf_start: *const u8, len: usize) -> String {
   let mut question_str = String::new();
   unsafe {
      // Skip the header
      const QNAME_TERMINATOR: u8 = 0;
      let mut dns_qname_qtype_qclass_walker = dns_buf_start.add(12);
      let dns_buf_end = dns_buf_start.add(len);
      while *dns_qname_qtype_qclass_walker != QNAME_TERMINATOR && dns_qname_qtype_qclass_walker != dns_buf_end {
         let segment_len = *dns_qname_qtype_qclass_walker as usize;
         if !question_str.is_empty() {
            question_str.push('.');
         }
         question_str
            .push_str(std::str::from_utf8(core::slice::from_raw_parts(dns_qname_qtype_qclass_walker.add(1), segment_len)).unwrap());
         dns_qname_qtype_qclass_walker = dns_qname_qtype_qclass_walker.add(1 + segment_len);
      }
   }

   question_str
}

#[test]
fn test_construct_query() {
   let domain = "www.google.com";
   let correct_query =
      [0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 3, 119, 119, 119, 6, 103, 111, 111, 103, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1];

   let mut query = vec![0; 512];
   let populated_query_slice = construct_query(domain, 1, &mut query);
   assert!(populated_query_slice == correct_query);
}
