use hcl::Body;
use std::fs;

fn main() {
    let contents = fs::read_to_string("./terraform/main.tf").expect("Failed to read file");
    let body: Body = hcl::from_str(contents.as_str()).unwrap();

    for block in body.blocks() {
        if block.identifier() == "resource" {
            let labels = block.labels();
            let resource_type = labels[0].as_str();
            println!("{resource_type:#?}");
        }
    }
}
