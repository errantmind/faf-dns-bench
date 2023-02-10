/*
FaF is a high performance DNS over TLS proxy
Copyright (C) 2022  James Bates

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

/// FaF DNS Proxy - Faster DNS Resolution
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
   /// debug, default: false
   #[clap(short, long)]
   #[clap(default_value_t = false)]
   pub debug: bool,

   /// num concurrent queries
   #[clap(short, long)]
   #[clap(default_value_t = 8)]
   pub concurrency: usize,

   /// n domains to include in the test
   #[clap(short, long)]
   #[clap(default_value_t = 250)]
   pub num_domains: usize,
}
