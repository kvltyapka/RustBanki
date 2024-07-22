use core::time::Duration;
use rand::Rng;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::RwLock;
use std::thread;
use std::time::Instant;

struct Bank {
    id: u32,
    balance: RwLock<i64>,
    accounts: RwLock<HashMap<u32, i64>>,
}

impl Bank {
    fn new(id: u32) -> Bank {
        Bank {
            id,
            balance: RwLock::new(0),
            accounts: RwLock::new(HashMap::new()),
        }
    }

    fn get_balance(&self, account_id: u32) -> i64 {
        thread::sleep(Duration::from_millis(500));
        let accounts = self.accounts.read().unwrap();
        *accounts.get(&account_id).unwrap_or(&0)
    }

    fn deposit(&self, account_id: u32, amount: i64) {
        thread::sleep(Duration::from_millis(500));
        let mut accounts = self.accounts.write().unwrap();
        let balance = accounts.entry(account_id).or_insert(0);
        *balance += amount;

        let mut bank_balance = self.balance.write().unwrap();
        *bank_balance += amount;
    }

    fn withdraw(&self, account_id: u32, amount: i64) -> bool {
        thread::sleep(Duration::from_millis(500));
        let mut accounts = self.accounts.write().unwrap();
        if let Some(balance) = accounts.get_mut(&account_id) {
            if *balance >= amount {
                *balance -= amount;

                let mut bank_balance = self.balance.write().unwrap();
                *bank_balance -= amount;
                return true;
            }
        }
        false
    }

    fn transfer(
        &self,
        to_bank: &Sender<Transaction>,
        from_account: u32,
        to_account: u32,
        amount: i64,
    ) -> bool {
        thread::sleep(Duration::from_millis(500));
        if self.withdraw(from_account, amount) {
            to_bank
                .send(Transaction::Deposit {
                    account_id: to_account,
                    amount,
                    response: mpsc::channel().0,
                })
                .unwrap();
            return true;
        }
        false
    }
}

enum Transaction {
    Deposit {
        account_id: u32,
        amount: i64,
        response: Sender<()>,
    },
    Withdraw {
        account_id: u32,
        amount: i64,
        response: Sender<()>,
    },
    Transfer {
        to_bank_id: u32,
        from_account: u32,
        to_account: u32,
        amount: i64,
        response: Sender<()>,
    },
    GetBalance {
        account_id: u32,
        response: Sender<i64>,
    },
}

fn bank_thread(
    bank: Bank,
    receiver: Receiver<Transaction>,
    banks: HashMap<u32, Sender<Transaction>>,
) {
    while let Ok(transaction) = receiver.recv() {
        match transaction {
            Transaction::Deposit {
                account_id,
                amount,
                response,
            } => {
                bank.deposit(account_id, amount);
                let _ = response.send(());
            }
            Transaction::Withdraw {
                account_id,
                amount,
                response,
            } => {
                let _ = bank.withdraw(account_id, amount);
                let _ = response.send(());
            }
            Transaction::Transfer {
                to_bank_id,
                from_account,
                to_account,
                amount,
                response,
            } => {
                if let Some(sender) = banks.get(&to_bank_id) {
                    let _ = bank.transfer(sender, from_account, to_account, amount);
                    let _ = response.send(());
                }
            }
            Transaction::GetBalance {
                account_id,
                response,
            } => {
                let balance = bank.get_balance(account_id);
                let _ = response.send(balance);
            }
        }
    }
}

fn request_balance(sender: &Sender<Transaction>, account_id: u32) -> i64 {
    let (response_tx, response_rx) = mpsc::channel();
    sender
        .send(Transaction::GetBalance {
            account_id,
            response: response_tx,
        })
        .unwrap();
    response_rx.recv().unwrap()
}

fn main() {
    let bank1 = Bank::new(1);
    let bank2 = Bank::new(2);
    let bank3 = Bank::new(3);

    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    let (tx3, rx3) = mpsc::channel();

    let mut banks = HashMap::new();
    banks.insert(1, tx1.clone());
    banks.insert(2, tx2.clone());
    banks.insert(3, tx3.clone());

    let banks1 = banks.clone();
    let handle1 = thread::spawn(move || {
        bank_thread(bank1, rx1, banks1);
    });

    let banks2 = banks.clone();
    let handle2 = thread::spawn(move || {
        bank_thread(bank2, rx2, banks2);
    });

    let banks3 = banks.clone();
    let handle3 = thread::spawn(move || {
        bank_thread(bank3, rx3, banks3);
    });

    let start = Instant::now();
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let transaction_type = rng.gen_range(0..3);
        let bank_id = rng.gen_range(1..=3);
        let account_id = rng.gen_range(1..=10);
        let amount = rng.gen_range(1..=1000);
        let (response_tx, response_rx) = mpsc::channel();
        match transaction_type {
            0 => {
                // Депозит
                let tx = banks.get(&bank_id).unwrap();
                println!(
                    "Депозит: {} рублей в банк {}, аккаунт {}",
                    amount, bank_id, account_id
                );
                tx.send(Transaction::Deposit {
                    account_id,
                    amount,
                    response: response_tx,
                })
                .unwrap();
            }
            1 => {
                // Снятие
                let tx = banks.get(&bank_id).unwrap();
                println!(
                    "Снятие: {} рублей из банка {}, аккаунт {}",
                    amount, bank_id, account_id
                );
                tx.send(Transaction::Withdraw {
                    account_id,
                    amount,
                    response: response_tx,
                })
                .unwrap();
            }
            2 => {
                // Перевод
                let to_bank_id = if bank_id == 1 { 2 } else { 1 };
                let to_account = rng.gen_range(1..=10);
                let tx = banks.get(&bank_id).unwrap();
                println!(
                    "Перевод: {} рублей из банка {}, аккаунт {} в банк {}, аккаунт {}",
                    amount, bank_id, account_id, to_bank_id, to_account
                );
                tx.send(Transaction::Transfer {
                    to_bank_id,
                    from_account: account_id,
                    to_account,
                    amount,
                    response: response_tx,
                })
                .unwrap();
            }
            _ => unreachable!(),
        }

        response_rx.recv().unwrap();
        thread::sleep(std::time::Duration::from_millis(10));
    }
    let duration = start.elapsed();
    println!("Все операции завершены за {:?}", duration);

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
}
