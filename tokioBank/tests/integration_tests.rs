#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use tokioBank::{Bank, Transaction};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_new_bank() {
        let bank = Bank::new(1);
        assert_eq!(bank.id, 1);
        assert_eq!(*bank.balance.read().await, 0);
        assert!(bank.accounts.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_balance() {
        let bank = Bank::new(1);
        bank.deposit(1, 100).await;
        let balance = bank.get_balance(1).await;
        assert_eq!(balance, 100);
    }

    #[tokio::test]
    async fn test_deposit() {
        let bank = Bank::new(1);
        bank.deposit(1, 100).await;
        let balance = bank.get_balance(1).await;
        assert_eq!(balance, 100);
        assert_eq!(*bank.balance.read().await, 100);
    }

    #[tokio::test]
    async fn test_withdraw() {
        let bank = Bank::new(1);
        bank.deposit(1, 100).await;
        let success = bank.withdraw(1, 50).await;
        assert!(success);
        let balance = bank.get_balance(1).await;
        assert_eq!(balance, 50);
        assert_eq!(*bank.balance.read().await, 50);
    }

    #[tokio::test]
    async fn test_withdraw_insufficient_funds() {
        let bank = Bank::new(1);
        bank.deposit(1, 100).await;
        let success = bank.withdraw(1, 150).await;
        assert!(!success);
        let balance = bank.get_balance(1).await;
        assert_eq!(balance, 100);
        assert_eq!(*bank.balance.read().await, 100);
    }

    #[tokio::test]
    async fn test_transfer() {
        let bank1 = Bank::new(1);
        let bank2 = Arc::new(RwLock::new(Bank::new(2))); // Lmao
        
        let (tx, mut rx) = mpsc::channel(32);
        let bank2_clone = Arc::clone(&bank2);

        tokio::spawn(async move {
            while let Some(transaction) = rx.recv().await {
                if let Transaction::Deposit { account_id, amount } = transaction {
                    bank2_clone.write().await.deposit(account_id, amount).await;
                }
            }
        });

        bank1.deposit(1, 100).await;
        let success = bank1.transfer(&tx, 1, 2, 50).await;
        assert!(success);

        let balance1 = bank1.get_balance(1).await;
        let balance2 = bank2.read().await.get_balance(2).await;

        assert_eq!(balance1, 50);
        assert_eq!(balance2, 50);
    }
}