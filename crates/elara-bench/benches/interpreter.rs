use elara_bench::{all_benchmarks, run_case};

fn main() {
    println!("benchmark,iterations,total_ns,ns_per_iter,result_count");
    for case in all_benchmarks() {
        let result = run_case(case).expect("benchmark case must execute successfully");
        let total_ns = result.elapsed.as_nanos();
        let ns_per_iter = total_ns / u128::from(result.iterations);
        println!(
            "{},{},{},{},{}",
            result.name, result.iterations, total_ns, ns_per_iter, result.last_result_count
        );
    }
}
