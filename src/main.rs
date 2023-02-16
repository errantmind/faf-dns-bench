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

#![allow(clippy::missing_safety_doc)]
#![feature(let_chains)]

mod args;
mod json_stats;
mod statics;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
   print_version();
   {
      // Handle `clear` argument

      if statics::ARGS.clear {
         json_stats::StatsSet::clear();
         println!("Stats Cleared.");
         return;
      }
   }

   assert!(statics::DOMAINS_TO_INCLUDE <= u16::MAX as usize, "You can not include more than 65535 (u16 MAX) domains");

   let domains = read_domains(statics::DOMAINS_TO_INCLUDE);
   let mut queries = Vec::with_capacity(statics::DOMAINS_TO_INCLUDE);
   let mut query_buf: Vec<u8> = vec![0; 512];

   // Pre-compute the DNS query for each domain
   for (id, domain) in domains.iter().enumerate() {
      queries.push(construct_query(domain.as_str(), id as u16, &mut query_buf).to_vec());
   }

   // Get dns_server to benchmark.
   let (dns_server, dns_port) = if let Some(server) = statics::ARGS.server.as_ref() {
      // Prefer options specified in ARGS.
      (server.to_string(), statics::ARGS.port)
   } else {
      // Regex for output in the form:
      // Server:         127.0.0.1
      // Address:        127.0.0.1#53
      let ip_and_port_regex: regex::Regex = regex::Regex::new(r"([0-9]{1,3}.[0-9]{1,3}.[0-9]{1,3}.[0-9]{1,3}#\d+)").unwrap();

      // Regex for output in the form (missing port on Windows):
      // Server:         127.0.0.1
      // Address:        127.0.0.1
      let ip_only_regex: regex::Regex = regex::Regex::new(r"([0-9]{1,3}.[0-9]{1,3}.[0-9]{1,3}.[0-9]{1,3})").unwrap();

      let nslookup_output = std::process::Command::new("nslookup").arg(".").output().expect("nslookup either doesn't exist or failed to start. Either install nslookup or use this program's options to manually specify a DNS server/port to benchmark");
      if let Some(ip_and_port)  = ip_and_port_regex.captures(std::str::from_utf8(&nslookup_output.stdout).unwrap()) && ip_and_port.len() > 0 {
         let first_capture_as_string = ip_and_port.get(0).unwrap().as_str().to_string();
         let server_and_port_split = first_capture_as_string.split('#').collect::<Vec<&str>>();
         (server_and_port_split[0].to_string(), server_and_port_split[1].parse::<u16>().unwrap())
      } else if let Some(ip_only) = ip_only_regex.captures(std::str::from_utf8(&nslookup_output.stdout).unwrap()) && ip_only.len() > 0 {
         (ip_only.get(0).unwrap().as_str().to_string(), statics::ARGS.port)
      } else {
         println!("Could not parse `nslookup .` output to determine your default DNS. Use this program's options to manually specify a DNS server/port to benchmark");
         return;
      }
   };

   println!("Benchmarking {dns_server}#{dns_port}\n");

   let mut local_udp_socket = mio::net::UdpSocket::bind("0.0.0.0:59233".parse().unwrap()).unwrap();
   let dns_addr = format!("{}:{}", dns_server.as_str(), dns_port);
   local_udp_socket.connect(dns_addr.parse().unwrap()).unwrap();

   let mut poll = mio::Poll::new().unwrap();
   const MIO_TOKEN: mio::Token = mio::Token(222);
   poll.registry().register(&mut local_udp_socket, MIO_TOKEN, mio::Interest::READABLE).unwrap();
   let mut events = mio::Events::with_capacity(340);

   let mut current_time = std::time::SystemTime::now();
   let mut timings_start = [(); statics::DOMAINS_TO_INCLUDE].map(|_| std::time::UNIX_EPOCH);
   let mut timings_end = [(); statics::DOMAINS_TO_INCLUDE].map(|_| std::time::UNIX_EPOCH);
   let mut current_query = 0;
   let mut completed_queries: usize = 0;
   let mut outstanding_queries: usize = 0;
   let mut finished_querying = false;
   let mut response_buf: Vec<u8> = vec![0; 512];

   loop {
      if completed_queries == statics::DOMAINS_TO_INCLUDE
         || std::time::SystemTime::now().duration_since(current_time).unwrap().as_millis() > statics::COLLECTION_TIMEOUT_MS
      {
         break;
      }

      if outstanding_queries < statics::MAX_CONCURRENCY && !finished_querying {
         for _ in 0..statics::MAX_CONCURRENCY - outstanding_queries {
            if current_query >= statics::DOMAINS_TO_INCLUDE {
               finished_querying = true;
               break;
            }

            timings_start[current_query] = std::time::SystemTime::now();
            local_udp_socket.send(&queries[current_query]).unwrap();
            current_query += 1;
            outstanding_queries += 1;
         }
      }
      let _ = poll.poll(&mut events, Some(std::time::Duration::from_millis(100)));

      for event in events.iter() {
         if event.token() == MIO_TOKEN {
            while let Ok(num_bytes_read) = local_udp_socket.recv(response_buf.as_mut_slice()) {
               assert!(num_bytes_read > 0 && num_bytes_read < 512);

               let id_from_response = u16::swap_bytes(unsafe { *(response_buf.as_ptr() as *const u16) });
               timings_end[id_from_response as usize] = std::time::SystemTime::now();
               let domain_str = get_question_as_string(response_buf.as_ptr(), num_bytes_read);
               if statics::ARGS.debug {
                  println!("{num_bytes_read:>4}b -> {domain_str}");
                  let response_slice = &response_buf[0..num_bytes_read];
                  println!("{response_slice:?}");
               }

               outstanding_queries -= 1;
               completed_queries += 1;
               current_time = std::time::SystemTime::now();
            }
         }
      }
   }

   {
      // Benchmark done, calculate stats
      let mut elapsed_ns = Vec::with_capacity(statics::DOMAINS_TO_INCLUDE);

      for i in 0..statics::DOMAINS_TO_INCLUDE {
         if timings_start[i] == std::time::UNIX_EPOCH || timings_end[i] == std::time::UNIX_EPOCH {
            continue;
         } else {
            elapsed_ns.push(timings_end[i].duration_since(timings_start[i]).unwrap().as_nanos());
         }
      }

      elapsed_ns.sort_unstable();

      let elapsed_ns_as_f64: Vec<f64> = elapsed_ns.iter().map(|x| *x as f64).collect();
      let median = elapsed_ns[elapsed_ns.len() / 2] as f64;
      let mean = statrs::statistics::Statistics::mean(&elapsed_ns_as_f64);
      let std_dev = statrs::statistics::Statistics::std_dev(&elapsed_ns_as_f64);
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

      json_stats::StatsSet::load()
         .push(json_stats::Stats {
            benchmark_name: statics::ARGS.bench.to_owned(),
            dns_server: format!("{dns_server}#{dns_port}"),
            n_samples: statics::DOMAINS_TO_INCLUDE,
            concurrency: statics::MAX_CONCURRENCY,
            median_ns: median,
            mean_ns: mean,
            stddev_ns: std_dev,
            min_ns: min,
            max_ns: max,
         })
         .save();
   }
}

fn print_version() {
   println!("{} v{} | repo: https://github.com/errantmind/faf-dns-bench\n", statics::PROJECT_NAME, statics::VERSION,);
}

/// Returns the bytes from the specified file. Unbuffered as there is no need atm. If large
/// files are added in the future (multiple GB), will need to add buffering.
#[inline(always)]
pub fn read_domains(num_domains_to_read: usize) -> Vec<String> {
   use std::io::BufRead;

   let file = std::fs::File::open(std::path::Path::new(statics::PROJECT_DIR).join("data").join("top-65535.csv")).unwrap();
   let reader = std::io::BufReader::with_capacity(1 << 15, file);
   let mut parsed_domains = Vec::with_capacity(num_domains_to_read);
   for (i, line) in reader.lines().enumerate() {
      if i < num_domains_to_read {
         parsed_domains.push(line.unwrap().trim().split(',').collect::<Vec<&str>>()[1].to_string());
      } else {
         break;
      }
   }

   parsed_domains
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
