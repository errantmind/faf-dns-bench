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

use clap::Parser;

/// FaF DNS Bench - A DNS Resolution Benchmarker
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
   /// enable debug output
   #[clap(short, long)]
   #[clap(default_value_t = false)]
   pub debug: bool,

   /// e.g. 1.1.1.1 [default: system default is parsed using `nslookup .`]
   #[clap(short, long)]
   pub server: Option<String>,

   #[clap(short, long)]
   #[clap(default_value_t = 53)]
   pub port: u16,
}
