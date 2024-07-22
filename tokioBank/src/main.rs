use futures::future::join_all;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{self, Duration, Instant};

struct Bank {
    id: u8,
    balance: RwLock<i64>,
    accounts: RwLock<HashMap<u8, i64>>,
}

impl Bank {
    fn new(id: u8) -> Bank {
        Bank {
            id,
            balance: RwLock::new(0),
            accounts: RwLock::new(HashMap::new()),
        }
    }

    async fn get_balance(&self, account_id: u8) -> i64 {
        time::sleep(Duration::from_millis(1000)).await;
        let accounts = self.accounts.read().await;
        *accounts.get(&account_id).unwrap_or(&0)
    }

    async fn deposit(&self, account_id: u8, amount: i64) {
        time::sleep(Duration::from_millis(1000)).await;
        let mut accounts = self.accounts.write().await;
        let balance = accounts.entry(account_id).or_insert(0);
        *balance += amount;

        let mut bank_balance = self.balance.write().await;
        *bank_balance += amount;
    }

    async fn withdraw(&self, account_id: u8, amount: i64) -> bool {
        time::sleep(Duration::from_millis(1000)).await;
        let mut accounts = self.accounts.write().await;
        if let Some(balance) = accounts.get_mut(&account_id) {
            if *balance >= amount {
                *balance -= amount;

                let mut bank_balance = self.balance.write().await;
                *bank_balance -= amount;
                return true;
            }
        }
        false
    }

    async fn transfer(
        &self,
        to_bank: &mpsc::Sender<Transaction>,
        from_account: u8,
        to_account: u8,
        amount: i64,
    ) -> bool {
        time::sleep(Duration::from_millis(1000)).await;
        if self.withdraw(from_account, amount).await {
            to_bank
                .send(Transaction::Deposit {
                    account_id: to_account,
                    amount,
                })
                .await
                .unwrap();
            return true;
        }
        false
    }
}

enum Transaction {
    Deposit {
        account_id: u8,
        amount: i64,
    },
    Withdraw {
        account_id: u8,
        amount: i64,
    },
    Transfer {
        to_bank_id: u8,
        from_account: u8,
        to_account: u8,
        amount: i64,
    },
    GetBalance {
        account_id: u8,
    },
}

async fn bank_thread(
    bank: Bank,
    mut receiver: mpsc::Receiver<Transaction>,
    banks: HashMap<u8, mpsc::Sender<Transaction>>,
) {
    while let Some(transaction) = receiver.recv().await {
        match transaction {
            Transaction::Deposit { account_id, amount } => {
                bank.deposit(account_id, amount).await;
            }
            Transaction::Withdraw { account_id, amount } => {
                let _ = bank.withdraw(account_id, amount).await;
            }
            Transaction::Transfer {
                to_bank_id,
                from_account,
                to_account,
                amount,
            } => {
                if let Some(sender) = banks.get(&to_bank_id) {
                    let _ = bank
                        .transfer(sender, from_account, to_account, amount)
                        .await;
                }
            }
            Transaction::GetBalance { account_id } => {
                let _ = bank.get_balance(account_id).await;
            }
        }
    }
}

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
