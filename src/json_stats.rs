#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct StatsSet {
   pub stats: Vec<Stats>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Stats {
   pub benchmark_name: String,
   pub dns_server: String,
   pub n_samples: usize,
   pub concurrency: usize,
   pub median_ns: f64,
   pub mean_ns: f64,
   pub stddev_ns: f64,
   pub min_ns: f64,
   pub max_ns: f64,
}

impl StatsSet {
   pub fn save(self) -> Self {
      let file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(Self::get_saved_results_path()).unwrap();
      let mut writer = std::io::BufWriter::new(file);
      std::io::Write::write_all(&mut writer, Self::serialize_to_json(&self).unwrap().as_bytes()).unwrap();
      std::io::Write::flush(&mut writer).unwrap();
      self
   }

   #[must_use]
   pub fn load() -> StatsSet {
      let mut file = match std::fs::File::open(std::path::Path::new(&Self::get_saved_results_path())) {
         Ok(f) => f,
         Err(_) => return Self::default(),
      };

      let file_len = file.metadata().unwrap().len();
      let mut bytes = Vec::with_capacity(file_len as usize + 1);
      std::io::Read::read_to_end(&mut file, &mut bytes).unwrap();
      Self::deserialize_from_json(&bytes).unwrap()
   }

   #[must_use]
   pub fn push(mut self, stats: Stats) -> Self {
      self.stats.push(stats);
      self
   }

   pub fn clear() {
      std::fs::remove_file(Self::get_saved_results_path()).unwrap();
   }

   fn get_saved_results_path() -> std::path::PathBuf {
      std::path::Path::new(crate::statics::PROJECT_DIR).join("data").join("saved_results.json")
   }

   fn serialize_to_json<T>(stats: &T) -> Option<String>
   where
      T: serde::Serialize,
   {
      serde_json::to_string(stats).ok()
   }

   fn deserialize_from_json<T>(bytes: &[u8]) -> Option<T>
   where
      T: serde::de::DeserializeOwned,
   {
      serde_json::from_slice(bytes).ok()
   }
}
