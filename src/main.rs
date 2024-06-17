#![allow(unused)]
// runtime modules
mod rt_transaction;
mod rt_executor;
mod rt_storage;

// in-memory data format modules
mod data_schema;
mod data_buffer;
mod data_parser_primitive;
mod data_parser_string;
mod data_parser_list;
mod data_parser_pair;

// file modules
mod storage_parquet;
mod storage_in_memory;

// physical op modules (sub-op in some literatures)
mod op_collect_hashmap;


#[cfg(test)]
#[ctor::ctor]
fn init() {
    muadb_util::init();
}

fn main() {
    println!("hello world");
}