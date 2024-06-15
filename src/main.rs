#![allow(unused)]

// standalone modules
mod util_bytes;
mod util_logging;

// sql modules
mod sql_parser;
mod util_pratt_parser;
mod sql_optimizer;
mod sql_compiler;

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

// physical op modules
mod op_collect_hashmap;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    crate::util_logging::init();
}

fn main() {
    println!("hello world");
}