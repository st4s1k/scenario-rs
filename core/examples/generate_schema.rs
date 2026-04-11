fn main() {
    let schema = scenario_rs_core::config::json_schema();
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
