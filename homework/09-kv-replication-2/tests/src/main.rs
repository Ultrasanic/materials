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

    /// Do not run model checking tests
    #[clap(long)]
    disable_mc_tests: bool,
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
    if !args.disable_mc_tests {
        tests.add("MC EMPTY SYSTEM", test_mc_empty_system, config);
        tests.add("MC BASIC", test_mc_basic, config);
    }
    tests.add("STALE REPLICA", test_stale_replica, config);
    tests.add("SLOPPY QUORUM", test_sloppy_quorum, config);
    if !args.disable_mc_tests {
        tests.add(
            "MC SLOPPY QUORUM HINTED HANDOFF",
            test_mc_sloppy_quorum_hinted_handoff,
            config,
        );
    }
    tests.add("CONCURRENT WRITES 1", test_concurrent_writes_1, config);
    tests.add("CONCURRENT WRITES 2", test_concurrent_writes_2, config);
    tests.add("CONCURRENT WRITES 3", test_concurrent_writes_3, config);
    if !args.disable_mc_tests {
        tests.add("MC CONCURRENT WRITES", test_mc_concurrent_writes, config);
    }
    tests.add("DIVERGED REPLICAS", test_diverged_replicas, config);
    tests.add("PARTITIONED CLIENTS", test_partitioned_clients, config);
    tests.add("SHOPPING CART 1", test_shopping_cart_1, config);
    tests.add("SHOPPING CART 2", test_shopping_cart_2, config);
    if !args.disable_mc_tests {
        tests.add("MC CONCURRENT CART", test_mc_concurrent_cart, config);
    }
    tests.add("SHOPPING XCART 1", test_shopping_xcart_1, config);
    tests.add("SHOPPING XCART 2", test_shopping_xcart_2, config);
    if !args.disable_mc_tests {
        tests.add("MC CONCURRENT XCART", test_mc_concurrent_xcart, config);
    }

    if args.test.is_none() {
        let (_, results) = tests.run();
        let score = score(results, args.disable_mc_tests);
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

fn score(results: IndexMap<String, TestResult>, disable_mc_tests: bool) -> f32 {
    let basic_group = string_set! {
        "BASIC", "MC EMPTY SYSTEM", "MC BASIC", "STALE REPLICA", "SLOPPY QUORUM", "MC SLOPPY QUORUM HINTED HANDOFF"
    };
    let concurrent_writes_group = string_set! {
        "CONCURRENT WRITES 1", "CONCURRENT WRITES 2", "CONCURRENT WRITES 3", "MC CONCURRENT WRITES"
    };
    let diverged_replicas_group = string_set! {"DIVERGED REPLICAS"};
    let partitioned_clients_group = string_set! {"PARTITIONED CLIENTS"};
    let shopping_cart_group = string_set! {
        "SHOPPING CART 1", "SHOPPING CART 2", "MC CONCURRENT CART"
    };
    let shopping_xcart_group = string_set! {
        "SHOPPING XCART 1", "SHOPPING XCART 2", "MC CONCURRENT XCART"
    };
    let all_groups = &(&(&(&(&basic_group | &concurrent_writes_group) | &diverged_replicas_group)
        | &partitioned_clients_group)
        | &shopping_cart_group)
        | &shopping_xcart_group;

    let mut all_results = results.clone();
    if disable_mc_tests {
        let result = TestResult::Err("Test in not run".to_string());
        let mc_results: IndexMap<String, TestResult> = all_groups
            .iter()
            .filter_map(|t| t.starts_with("MC ").then_some((t.clone(), result.clone())))
            .collect();
        all_results.extend(mc_results);
    }

    assert_eq!(all_groups, all_results.keys().cloned().collect::<IndexSet<String>>());

    let mut passed = IndexSet::new();
    for (test, result) in all_results {
        if result.is_ok() {
            passed.insert(test);
        }
    }

    let mut score = 0.;
    print!("BASIC tests: ");
    if basic_group.is_subset(&passed) {
        score += 4.;
        println!("passed");
    } else {
        println!("not passed {:?}", basic_group.difference(&passed));
    }
    print!("CONCURRENT WRITES tests: ");
    if concurrent_writes_group.is_subset(&passed) {
        score += 1.;
        println!("passed");
    } else {
        println!(
            "not passed {:?}",
            concurrent_writes_group.difference(&passed)
        );
    }
    print!("DIVERGED REPLICAS test: ");
    if diverged_replicas_group.is_subset(&passed) {
        score += 1.;
        println!("passed");
    } else {
        println!(
            "not passed {:?}",
            diverged_replicas_group.difference(&passed)
        );
    }
    print!("PARTITIONED CLIENTS test: ");
    if partitioned_clients_group.is_subset(&passed) {
        score += 1.;
        println!("passed");
    } else {
        println!(
            "not passed {:?}",
            partitioned_clients_group.difference(&passed)
        );
    }
    print!("SHOPPING CART tests: ");
    if shopping_cart_group.is_subset(&passed) {
        score += 1.;
        println!("passed");
    } else {
        println!("not passed {:?}", shopping_cart_group.difference(&passed));
    }
    print!("SHOPPING XCART tests: ");
    if shopping_xcart_group.is_subset(&passed) {
        score += 2.;
        println!("passed");
    } else {
        println!("not passed {:?}", shopping_xcart_group.difference(&passed));
    }
    if basic_group.is_subset(&passed) {
        score
    } else {
        0.
    }
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
