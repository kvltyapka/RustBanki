use futures::future::join_all;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};
use tokioBank::{Bank, Transaction, bank_thread};


#[tokio::main]
async fn main() {
    let bank1 = Bank::new(1);
    let bank2 = Bank::new(2);
    let bank3 = Bank::new(3);

    let (tx1, rx1) = mpsc::channel(32);
    let (tx2, rx2) = mpsc::channel(32);
    let (tx3, rx3) = mpsc::channel(32);

    let mut banks = HashMap::new();
    banks.insert(1, tx1.clone());
    banks.insert(2, tx2.clone());
    banks.insert(3, tx3.clone());

    let banks1 = banks.clone();
    tokio::spawn(async move {
        bank_thread(bank1, rx1, banks1).await;
    });

    let banks2 = banks.clone();
    tokio::spawn(async move {
        bank_thread(bank2, rx2, banks2).await;
    });

    let banks3 = banks.clone();
    tokio::spawn(async move {
        bank_thread(bank3, rx3, banks3).await;
    });

    let start = Instant::now();
    let mut rng = StdRng::from_entropy();
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let transaction_type = rng.gen_range(0..3);
        let bank_id = rng.gen_range(1..=3);
        let account_id = rng.gen_range(1..=10);
        let amount = rng.gen_range(1..=1000);
        let tx = banks.get(&bank_id).unwrap().clone();
        let mut rng = StdRng::from_entropy();

        let task = tokio::spawn(async move {
            match transaction_type {
                0 => {
                    println!(
                        "Депозит: {} рублей в банк {}, аккаунт {}",
                        amount, bank_id, account_id
                    );
                    tx.send(Transaction::Deposit { account_id, amount })
                        .await
                        .unwrap();
                }
                1 => {
                    println!(
                        "Снятие: {} рублей из банка {}, аккаунт {}",
                        amount, bank_id, account_id
                    );
                    tx.send(Transaction::Withdraw { account_id, amount })
                        .await
                        .unwrap();
                }
                2 => {
                    let to_bank_id = if bank_id == 1 { 2 } else { 1 };
                    let to_account = rng.gen_range(1..=10);
                    println!(
                        "Перевод: {} рублей из банка {}, аккаунт {} в банк {}, аккаунт {}",
                        amount, bank_id, account_id, to_bank_id, to_account
                    );
                    tx.send(Transaction::Transfer {
                        to_bank_id,
                        from_account: account_id,
                        to_account,
                        amount,
                    })
                    .await
                    .unwrap();
                }
                _ => unreachable!(),
            }
        });
        tasks.push(task);
    }

    let when_timeout = time::sleep(Duration::from_secs(10));
    tokio::pin!(when_timeout);

    loop {
        tokio::select! {
            _ = &mut when_timeout => {
                println!("Таймаут");
                break;
            }
            _ = join_all(tasks) => {
                println!("Готово");
                let duration = start.elapsed();
                println!("Все операции завершены за {:?}", duration);
                break;
            }
        }
    }
}
