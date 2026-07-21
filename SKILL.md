# cdp_rs v4.2 - Modular Framework with Database

## Модульная структура

```
cdp_rs/
├── src/
│   ├── lib.rs                    # Библиотека
│   │
│   ├── email/                    # 📧 Email модуль
│   │   ├── smtp/
│   │   ├── validator/
│   │   └── endorsements/
│   │
│   ├── analysis/                 # 📊 Analysis модуль
│   │   ├── critic/
│   │   ├── integrator/
│   │   └── validator/
│   │
│   ├── database/                 # 🗄️ Database модуль (НОВЫЙ)
│   │   ├── sqlite/
│   │   │   ├── connection.rs     # Подключение
│   │   │   ├── query.rs          # Запросы
│   │   │   └── error.rs          # Ошибки
│   │   ├── pool/
│   │   │   └── manager.rs        # Пул соединений
│   │   └── models/
│   │       ├── user.rs           # Модель пользователя
│   │       ├── email.rs          # Модель email
│   │       └── endorsement.rs    # Модель endorsement
│   │
│   ├── browser/                  # 🌐 Browser модуль
│   └── page/                     # 📄 Page модуль
```

## Использование Database модуля

```rust
use cdp_rs::database::{Database, User, Email, EndorsementRecord};

// Создание БД
let db = Database::create("app.db")?;

// Создание таблиц
db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")?;
db.execute("CREATE TABLE emails (id INTEGER PRIMARY KEY, from_addr TEXT, to_addr TEXT)")?;
db.execute("CREATE TABLE endorsements (id INTEGER PRIMARY KEY, user_id INTEGER, category TEXT)")?;

// Вставка данных
let user = User::new(1, "Yuriy", "apohob5@gmail.com", "math.LO");
db.execute(&user.to_sql())?;

let email = Email::new(1, "from@gmail.com", "to@gmail.com", "Subject", "Body");
db.execute(&email.to_sql())?;

let endorsement = EndorsementRecord::new(1, 1, "math.LO", "NWTCV4");
db.execute(&endorsement.to_sql())?;

// Запросы
let result = db.query("SELECT * FROM users")?;
println!("Found {} users", result.len());
```

## Структура модулей

```
database/
├── mod.rs              # Точка входа
├── sqlite/
│   ├── mod.rs
│   ├── connection.rs   # Подключение к SQLite
│   ├── query.rs        # Результаты запросов
│   └── error.rs        # Обработка ошибок
├── pool/
│   ├── mod.rs
│   └── manager.rs      # Пул соединений
└── models/
    ├── mod.rs
    ├── user.rs         # Пользователи
    ├── email.rs        # Email сообщения
    └── endorsement.rs  # Endorsement записи
```

---
*cdp_rs v4.2 - Модульная архитектура с базой данных*
