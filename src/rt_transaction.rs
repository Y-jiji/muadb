// no-control flow, one-after-one execution
pub struct Transaction {
}

// stream and collection instructions
pub enum Operator {
    Filter,
    Map,
    Split,
    Iterate,
    CollectSet,
    CollectMap,
}

