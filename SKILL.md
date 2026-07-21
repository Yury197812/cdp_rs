# cdp_rs v4.1 - Modular Framework with Analysis

## Модульная структура

```
cdp_rs/
├── src/
│   ├── lib.rs                    # Библиотека
│   ├── main.rs                   # CLI
│   │
│   ├── email/                    # 📧 Email модуль
│   │   ├── smtp/
│   │   │   ├── client.rs         # SMTP клиент
│   │   │   └── message.rs        # Конструктор писем
│   │   ├── validator/
│   │   │   ├── dns.rs            # DNS валидация
│   │   │   └── smtp_check.rs     # SMTP проверка
│   │   └── endorsements/
│   │       ├── endorsers.rs      # Списки эндорсеров
│   │       └── sender.rs         # Отправка писем
│   │
│   ├── analysis/                 # 📊 Analysis модуль
│   │   ├── critic/
│   │   │   ├── engine.rs         # Движок критики
│   │   │   └── rules.rs          # Правила критики
│   │   ├── integrator/
│   │   │   ├── merger.rs         # Слияние данных
│   │   │   └── transformer.rs    # Трансформация данных
│   │   └── validator/
│   │       ├── types.rs          # Типы валидации
│   │       └── rules.rs          # Правила валидации
│   │
│   ├── browser/                  # 🌐 Browser модуль
│   └── page/                     # 📄 Page модуль
```

## Использование

### Email модуль
```rust
use cdp_rs::email::{SmtpClient, send_endorsements, get_physicist_endorsers};

let mut smtp = SmtpClient::new("smtp.gmail.com", 587)?;
smtp.auth("user@gmail.com", "pass")?;

let endorsers = get_physicist_endorsers();
let (sent, failed) = send_endorsements(&mut smtp, &endorsers, "from@gmail.com");
```

### Analysis модуль
```rust
use cdp_rs::analysis::{Critic, validate_input, integrate_data};
use cdp_rs::analysis::critic::rules::{CritiqueRule, CritiqueResult};
use cdp_rs::analysis::validator::rules::ValidationRule;

// Критический анализ
let mut critic = Critic::new();
critic.add_rule(CritiqueRule::LogicCheck);
let result = critic.analyze("if x > 0 then return true");

// Валидация ввода
let validation = validate_input("test@email.com", &[
    ValidationRule::NotEmpty,
    ValidationRule::EmailFormat,
]);

// Интеграция данных
let mut sources = Vec::new();
sources.insert("key1".to_string(), "value1".to_string());
let integrated = integrate_data(sources);
```

---
*cdp_rs v4.1 - Модульная архитектура с анализом*
