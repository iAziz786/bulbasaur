use std::{collections::HashMap, error::Error};

use crate::cli_config::CliConfig;

use serde::{Deserialize, Serialize};

use csv::Trim;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Transaction {
    r#type: String,
    client: u16,
    amount: Option<f32>,
    tx: u32,
}

#[derive(Debug)]
struct Record {
    txn: Transaction,
    has_dispute: bool,
}

#[derive(Serialize, Clone, Debug)]
struct Stats {
    client: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

pub fn run(config: CliConfig) -> Result<(), Box<dyn Error>> {
    let mut stats = HashMap::new();
    let mut records = vec![];

    let mut rdr = csv::ReaderBuilder::new()
        .trim(Trim::All)
        // from_path buffers by default, so we can expect the stream of data
        // rather than loading the entire data in the memory.
        .from_path(config.filename)?;

    for transaction in rdr.deserialize() {
        let txn: Transaction = transaction?;
        process_txn(txn, &mut records, &mut stats);
    }

    let mut wtr = csv::Writer::from_writer(vec![]);
    for (_, stat) in stats.iter() {
        wtr.serialize(stat)?;
    }
    let data = String::from_utf8(wtr.into_inner()?)?;
    print!("{}", data);
    Ok(())
}

fn process_txn(txn: Transaction, records: &mut Vec<Record>, stats: &mut HashMap<u16, Stats>) {
    let pos = match records.binary_search_by(|record: &Record| record.txn.tx.cmp(&txn.tx)) {
        Ok(pos) => pos,
        Err(pos) => pos,
    };
    // no need to add the dispute, resolve, and chargeback transactions
    if txn.r#type == "deposit" || txn.r#type == "withdrawal" {
        records.insert(
            pos,
            Record {
                // TODO: make it unclone
                txn: txn.clone(),
                has_dispute: false,
            },
        );
    }

    match txn.r#type.as_str() {
        "deposit" => {
            let entry = stats.entry(txn.client).or_insert(Stats {
                client: txn.client,
                available: 0.0,
                held: 0.0,
                total: 0.0,
                locked: false,
            });
            if let Some(amount) = txn.amount {
                entry.available = round(entry.available + amount);
                entry.total = round(entry.total + amount);
            }
        }
        "withdrawal" => {
            if let Some(entry) = stats.get_mut(&txn.client) {
                if let Some(amount) = txn.amount {
                    // insufficient balance check
                    if (entry.total - amount) < 0.0 {
                        return;
                    }
                    entry.available = round(entry.available - amount);
                    entry.total = round(entry.total - amount);
                }
            }
        }
        "dispute" => {
            if let Some(entry) = stats.get_mut(&txn.client) {
                match records.binary_search_by(|record| record.txn.tx.cmp(&txn.tx)) {
                    Ok(pos) => {
                        if let Some(prev_record) = records.get_mut(pos) {
                            prev_record.has_dispute = true;
                            match prev_record.txn.r#type.as_str() {
                                "deposit" => {
                                    entry.available =
                                        round(entry.available - prev_record.txn.amount.unwrap());
                                    entry.held =
                                        round(entry.held + prev_record.txn.amount.unwrap());
                                }
                                "withdrawal" => {
                                    entry.available =
                                        round(entry.available + prev_record.txn.amount.unwrap());
                                    entry.held =
                                        round(entry.held - prev_record.txn.amount.unwrap());
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_pos) => {}
                }
            }
        }
        "resolve" => {
            if let Some(entry) = stats.get_mut(&txn.client) {
                match records.binary_search_by(|record| record.txn.tx.cmp(&txn.tx)) {
                    Ok(pos) => {
                        if let Some(prev_record) = records.get_mut(pos) {
                            if prev_record.has_dispute {
                                match prev_record.txn.r#type.as_str() {
                                    "deposit" => {
                                        entry.available = round(
                                            entry.available + prev_record.txn.amount.unwrap(),
                                        );
                                        entry.held =
                                            round(entry.held - prev_record.txn.amount.unwrap());
                                    }
                                    "withdrawal" => {
                                        entry.available = round(
                                            entry.available - prev_record.txn.amount.unwrap(),
                                        );
                                        entry.held =
                                            round(entry.held + prev_record.txn.amount.unwrap());
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Err(_pos) => {}
                }
            }
        }
        "chargeback" => {
            if let Some(entry) = stats.get_mut(&txn.client) {
                match records.binary_search_by(|record| record.txn.tx.cmp(&txn.tx)) {
                    Ok(pos) => {
                        if let Some(prev_record) = records.get_mut(pos) {
                            if prev_record.has_dispute {
                                match prev_record.txn.r#type.as_str() {
                                    "deposit" => {
                                        entry.total =
                                            round(entry.total - prev_record.txn.amount.unwrap());
                                        entry.held =
                                            round(entry.held - prev_record.txn.amount.unwrap());
                                    }
                                    "withdrawal" => {
                                        entry.total =
                                            round(entry.total + prev_record.txn.amount.unwrap());
                                        entry.held =
                                            round(entry.held + prev_record.txn.amount.unwrap());
                                    }
                                    _ => {}
                                }
                                entry.locked = true;
                            }
                        }
                    }
                    Err(_pos) => {}
                }
            }
        }
        _ => {}
    }
}

/// round is used for four digit precision
fn round(amount: f32) -> f32 {
    (amount * 10000_f32).round() / 10000_f32
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, vec};

    use crate::app::process_txn;

    use super::Transaction;

    #[test]
    fn should_deposit_correctly() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(1.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(1.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats)
        }

        assert_eq!(stats.get(&1).unwrap().available, 2.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 2.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_withdrawal_correctly() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(1.0),
                client: 1,
                tx: 1,
                r#type: "withdrawal".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats)
        }

        assert_eq!(stats.get(&1).unwrap().available, 4.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 4.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_withdrawal_fail_when_insufficient_fund() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(6.0),
                client: 1,
                tx: 1,
                r#type: "withdrawal".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats)
        }

        assert_eq!(stats.get(&1).unwrap().available, 5.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 5.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_dispute_deposit_txn_correctly() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(1.0),
                client: 1,
                tx: 2,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "dispute".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats)
        }

        assert_eq!(stats.get(&1).unwrap().available, 5.0);
        assert_eq!(stats.get(&1).unwrap().held, 1.0);
        assert_eq!(stats.get(&1).unwrap().total, 6.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_dispute_withdrawal_txn_correctly() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(1.0),
                client: 1,
                tx: 2,
                r#type: "withdrawal".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "dispute".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats)
        }

        assert_eq!(stats.get(&1).unwrap().available, 5.0);
        assert_eq!(stats.get(&1).unwrap().held, -1.0);
        assert_eq!(stats.get(&1).unwrap().total, 4.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_resolve_work_for_deposit_txn() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(2.0),
                client: 1,
                tx: 2,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "dispute".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "resolve".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 7.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 7.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_process_deposit_chargeback() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(2.0),
                client: 1,
                tx: 2,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "dispute".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "chargeback".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 5.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 5.0);
        assert!(stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_process_withdrawal_chargeback() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(2.0),
                client: 1,
                tx: 2,
                r#type: "withdrawal".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "dispute".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "chargeback".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 5.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 5.0);
        assert!(stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_ignore_resolve_if_no_dispute_deposit_txn() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(3.0),
                client: 1,
                tx: 2,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "resolve".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 8.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 8.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_ignore_resolve_if_no_dispute_withdrawal_txn() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(3.0),
                client: 1,
                tx: 2,
                r#type: "withdrawal".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "resolve".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 2.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 2.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_ignore_chargeback_if_no_dispute_deposit_txn() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(3.0),
                client: 1,
                tx: 2,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "chargeback".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 8.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 8.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_ignore_chargeback_if_no_dispute_withdrawal_txn() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.0),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(3.0),
                client: 1,
                tx: 2,
                r#type: "withdrawal".to_string(),
            },
            Transaction {
                amount: None,
                client: 1,
                tx: 2,
                r#type: "chargeback".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 2.0);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 2.0);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_accept_amount_till_four_digits_for_deposit() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.66666),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(5.66666),
                client: 1,
                tx: 2,
                r#type: "deposit".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 11.3334);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 11.3334);
        assert!(!stats.get(&1).unwrap().locked);
    }

    #[test]
    fn should_accept_amount_till_four_digits_for_withdrawal() {
        let mut stats = HashMap::new();
        let txns = vec![
            Transaction {
                amount: Some(5.66666),
                client: 1,
                tx: 1,
                r#type: "deposit".to_string(),
            },
            Transaction {
                amount: Some(5.11111),
                client: 1,
                tx: 2,
                r#type: "withdrawal".to_string(),
            },
        ];
        let records = &mut vec![];
        for txn in txns {
            process_txn(txn, records, &mut stats);
        }

        assert_eq!(stats.get(&1).unwrap().available, 0.5556);
        assert_eq!(stats.get(&1).unwrap().held, 0.0);
        assert_eq!(stats.get(&1).unwrap().total, 0.5556);
        assert!(!stats.get(&1).unwrap().locked);
    }
}
