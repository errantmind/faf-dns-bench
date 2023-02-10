/// Returns the bytes from the specified file. Unbuffered as there is no need atm. If large
/// files are added in the future (multiple GB), will need to add buffering.
#[inline(always)]
pub fn read_domains(num_domains_to_read: usize) -> Vec<String> {
   use std::io::BufRead;

   let file = std::fs::File::open(std::path::Path::new(crate::statics::PROJECT_DIR).join("data").join("top-65535.csv")).unwrap();
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
