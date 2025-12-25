mod common;
mod tests;
mod tests_mc;

use std::env;
use std::io::Write;

use clap::Parser;
use env_logger::Builder;
use indexmap::{IndexMap, IndexSet};
use log::LevelFilter;

use anysystem::python::PyProcessFactory;
use anysystem::test::{TestResult, TestSuite};

use crate::common::TestConfig;
use crate::tests::*;
use crate::tests_mc::*;

/// Replicated KV Store Homework Tests
#[derive(Parser, Debug)]
#[clap(about, long_about = None)]
struct Args {
    /// Path to Python file with solution
    #[clap(long = "impl", short = 'i', default_value = "solution/node.py")]
    solution_path: String,

    /// Test to run (optional)
    #[clap(long = "test", short)]
    test: Option<String>,

    /// Print execution trace
    #[clap(long, short)]
    debug: bool,

    /// Number of nodes used in tests
    #[clap(long, short, default_value = "6")]
    node_count: u32,

    /// Random seed used in tests
    #[clap(long, short, default_value = "123")]
    seed: u64,
}

fn main() {
    let args = Args::parse();
    if args.debug {
        Builder::new()
            .filter(Some("anysystem"), LevelFilter::Debug)
            .format(|buf, record| writeln!(buf, "{}", record.args()))
            .init();
    }
    append_to_python_path("../../anysystem/python".to_string());
    env::set_var("PYTHONHASHSEED", args.seed.to_string());
    let proc_factory = PyProcessFactory::new(&args.solution_path, "StorageNode");
    let config = TestConfig {
        impl_path: &args.solution_path,
        proc_factory: &proc_factory,
        proc_count: args.node_count,
        seed: args.seed,
    };

    let mut tests = TestSuite::new();
    tests.add("BASIC", test_basic, config);
    tests.add("REPLICAS CHECK", test_replicas_check, config);
    tests.add("CONCURRENT GET PUT", test_concurrent_get_put, config);
    tests.add("MC BASIC", test_mc_basic, config);
    tests.add("CONCURRENT WRITES", test_concurrent_writes, config);
    tests.add("CONCURRENT WRITES TIE", test_concurrent_writes_tie, config);
    tests.add("MC CONCURRENT WRITES", test_mc_concurrent_writes, config);
    tests.add("STALE REPLICA", test_stale_replica, config);
    tests.add("STALE REPLICA DELETE", test_stale_replica_delete, config);
    tests.add("DIVERGED REPLICAS", test_diverged_replicas, config);
    tests.add("SLOPPY QUORUM READ", test_sloppy_quorum_read, config);
    tests.add("SLOPPY QUORUM WRITE", test_sloppy_quorum_write, config);
    tests.add("SLOPPY QUORUM TRICKY", test_sloppy_quorum_tricky, config);
    tests.add(
        "MC SLOPPY QUORUM HINTED HANDOFF",
        test_mc_sloppy_quorum_hinted_handoff,
        config,
    );
    tests.add("PARTITION CLIENTS", test_partition_clients, config);
    tests.add("PARTITION MIXED", test_partition_mixed, config);

    if args.test.is_none() {
        let (_, results) = tests.run();
        let score = score(results);
        println!("\nSCORE: {score}\n");
    } else {
        tests.run_test(&args.test.unwrap());
    }
}

macro_rules! string_set {
    ($($x:expr),+ $(,)?) => (
        IndexSet::from([$($x.to_string()),+])
    );
}

fn score(results: IndexMap<String, TestResult>) -> f32 {
    let basic_group = string_set! {
        "BASIC", "REPLICAS CHECK", "CONCURRENT GET PUT", "MC BASIC"
    };
    let conflict_resolution_group = string_set! {
        "CONCURRENT WRITES", "CONCURRENT WRITES TIE", "MC CONCURRENT WRITES",
        "STALE REPLICA", "STALE REPLICA DELETE", "DIVERGED REPLICAS"
    };
    let sloppy_partition_group = string_set! {
        "SLOPPY QUORUM READ", "SLOPPY QUORUM WRITE", "SLOPPY QUORUM TRICKY", "MC SLOPPY QUORUM HINTED HANDOFF",
        "PARTITION CLIENTS", "PARTITION MIXED"
    };
    let all_groups = &(&basic_group | &conflict_resolution_group) | &sloppy_partition_group;
    assert_eq!(all_groups, results.keys().cloned().collect::<IndexSet<String>>());

    let mut passed = IndexSet::new();
    for (test, result) in results {
        if result.is_ok() {
            passed.insert(test);
        }
    }

    let mut score = 0.;
    print!("Basic tests: ");
    if basic_group.is_subset(&passed) {
        score += 3.;
        println!("passed");
    } else {
        println!("not passed {:?}", basic_group.difference(&passed));
    }
    print!("Conflict resolution tests: ");
    if conflict_resolution_group.is_subset(&passed) {
        if basic_group.is_subset(&passed) {
            score += 3.;
        }
        println!("passed");
    } else {
        println!(
            "not passed {:?}",
            conflict_resolution_group.difference(&passed)
        );
    }
    print!("Sloppy quorum & network partition tests: ");
    if sloppy_partition_group.is_subset(&passed) {
        if basic_group.is_subset(&passed) {
            score += 3.;
        }
        println!("passed");
    } else {
        println!(
            "not passed {:?}",
            sloppy_partition_group.difference(&passed)
        );
    }
    score
}

fn append_to_python_path(entry: String) {
    let path_separator = if cfg!(windows) { ";" } else { ":" };
    let current_path = env::var("PYTHONPATH").unwrap_or_default();
    let updated_path = if current_path.is_empty() {
        entry
    } else {
        format!("{current_path}{path_separator}{entry}")
    };
    env::set_var("PYTHONPATH", updated_path);
}
