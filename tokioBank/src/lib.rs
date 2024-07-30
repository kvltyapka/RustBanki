use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{self, Duration};

/// Структура, представляющая банк.
pub struct Bank {
    /// Идентификатор банка.
    pub id: u8,
    /// Общий баланс банка.
    pub balance: RwLock<i64>,
    /// Счета клиентов банка.
    pub accounts: RwLock<HashMap<u8, i64>>,
}

impl Bank {
    /// Создает новый банк с заданным идентификатором.
    ///
    /// # Аргументы
    ///
    /// * `id` - Идентификатор банка.
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр структуры `Bank`.
    pub fn new(id: u8) -> Bank {
        Bank {
            id,
            balance: RwLock::new(0),
            accounts: RwLock::new(HashMap::new()),
        }
    }

    /// Получает баланс указанного счета.
    ///
    /// # Аргументы
    ///
    /// * `account_id` - Идентификатор счета.
    ///
    /// # Возвращает
    ///
    /// Баланс счета.
    pub async fn get_balance(&self, account_id: u8) -> i64 {
        time::sleep(Duration::from_millis(500)).await;
        let accounts = self.accounts.read().await;
        *accounts.get(&account_id).unwrap_or(&0)
    }

    /// Вносит указанную сумму на указанный счет.
    ///
    /// # Аргументы
    ///
    /// * `account_id` - Идентификатор счета.
    /// * `amount` - Сумма для внесения.
    pub async fn deposit(&self, account_id: u8, amount: i64) {
        time::sleep(Duration::from_millis(500)).await;
        let mut accounts = self.accounts.write().await;
        let balance = accounts.entry(account_id).or_insert(0);
        *balance += amount;

        let mut bank_balance = self.balance.write().await;
        *bank_balance += amount;
    }

    /// Снимает указанную сумму с указанного счета.
    ///
    /// # Аргументы
    ///
    /// * `account_id` - Идентификатор счета.
    /// * `amount` - Сумма для снятия.
    ///
    /// # Возвращает
    ///
    /// `true`, если операция успешна, иначе `false`.
    pub async fn withdraw(&self, account_id: u8, amount: i64) -> bool {
        time::sleep(Duration::from_millis(500)).await;
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

    /// Переводит указанную сумму с одного счета на другой.
    ///
    /// # Аргументы
    ///
    /// * `to_bank` - Канал для отправки транзакции в другой банк.
    /// * `from_account` - Идентификатор счета отправителя.
    /// * `to_account` - Идентификатор счета получателя.
    /// * `amount` - Сумма для перевода.
    ///
    /// # Возвращает
    ///
    /// `true`, если операция успешна, иначе `false`.
    pub async fn transfer(
        &self,
        to_bank: &mpsc::Sender<Transaction>,
        from_account: u8,
        to_account: u8,
        amount: i64,
    ) -> bool {
        time::sleep(Duration::from_millis(500)).await;
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

/// Перечисление, представляющее различные типы транзакций.
pub enum Transaction {
    /// Транзакция для внесения средств.
    Deposit {
        account_id: u8,
        amount: i64,
    },
    /// Транзакция для снятия средств.
    Withdraw {
        account_id: u8,
        amount: i64,
    },
    /// Транзакция для перевода средств.
    Transfer {
        to_bank_id: u8,
        from_account: u8,
        to_account: u8,
        amount: i64,
    },
    /// Транзакция для получения баланса.
    GetBalance {
        account_id: u8,
    },
}

/// Функция, представляющая поток банка для обработки транзакций.
///
/// # Аргументы
///
/// * `bank` - Экземпляр банка.
/// * `receiver` - Канал для получения транзакций.
/// * `banks` - Хэш-карта всех банков и их каналов.
pub async fn bank_thread(
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