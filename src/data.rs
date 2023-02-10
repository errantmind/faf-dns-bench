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
