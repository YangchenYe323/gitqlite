use std::{collections::VecDeque, fs, hint::black_box, time::Duration};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn ignore_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("Apply gitignore rules to all files under a directory");

  group
    .measurement_time(Duration::from_secs(100))
    .bench_function(BenchmarkId::new("Baseline (Do nothing)", ""), |bencher| {
      let repo_root = gitqlite::git::utils::find_gitqlite_root(std::env::current_dir().unwrap()).unwrap();
      let gitqlite_home = repo_root.join(".gitqlite");
      let git_home = repo_root.join(".git");

      bencher.iter(|| {
          let mut queue = VecDeque::new();
          queue.push_back(repo_root.clone());
          while let Some(cur_directory) = queue.pop_front() {
            if cur_directory.starts_with(&gitqlite_home) || cur_directory.starts_with(&git_home) {
              continue
            }
            for entry in fs::read_dir(&cur_directory).unwrap().filter_map(Result::ok) {
              let path = entry.path();
              if path.is_dir() {
                queue.push_back(path);
              }
            }
          }
      })
    })
    .bench_function(BenchmarkId::new("GitIgnore V1", ""), |bencher| {
      let repo_root = gitqlite::git::utils::find_gitqlite_root(std::env::current_dir().unwrap()).unwrap();
      let gitqlite_home = repo_root.join(".gitqlite");
      let git_home = repo_root.join(".git");
      let ignore = gitqlite::git::ignore::read_gitignore(repo_root.clone()).unwrap();

      bencher.iter(|| {
          let mut queue = VecDeque::new();
          queue.push_back(repo_root.clone());
          while let Some(cur_directory) = queue.pop_front() {
            if cur_directory.starts_with(&gitqlite_home) || cur_directory.starts_with(&git_home) {
              continue
            }
            for entry in fs::read_dir(&cur_directory).unwrap().filter_map(Result::ok) {
              let path = entry.path();
              if ignore.should_ignore(&path) {
                continue;
              }
              if path.is_dir() {
                queue.push_back(path);
              }
            }
          }
      })
  });

}

criterion_group!(benches, ignore_benchmark);
criterion_main!(benches);