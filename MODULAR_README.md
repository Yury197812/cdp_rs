# cdp_rs v4.0 - Modular Browser Automation Framework

## Модульная структура

```
cdp_rs/
├── src/
│   ├── lib.rs                    # Библиотека
│   ├── main.rs                   # CLI
│   │
│   ├── browser/                  # 🌐 Browser модуль
│   │   ├── mod.rs
│   │   ├── manager.rs            # Управление браузером
│   │   ├── pool.rs               # Пул браузеров
│   │   └── cdp.rs                # CDP клиент
│   │
│   ├── page/                     # 📄 Page модуль
│   │   ├── mod.rs
│   │   ├── navigation.rs         # Навигация
│   │   ├── interaction.rs        # Взаимодействие
│   │   └── screenshot.rs         # Скриншоты
│   │
│   ├── email/                    # 📧 Email модуль
│   │   ├── mod.rs
│   │   ├── smtp/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs         # SMTP клиент
│   │   │   └── message.rs        # Конструктор писем
│   │   ├── validator/
│   │   │   ├── mod.rs
│   │   │   ├── dns.rs            # DNS валидация
│   │   │   └── smtp_check.rs     # SMTP проверка
│   │   └── endorsements/
│   │       ├── mod.rs
│   │       ├── endorsers.rs      # Списки эндорсеров
│   │       └── sender.rs         # Отправка писем
│   │
│   └── bin/                      # 🚀 Бинарники
│       └── send_physicists.rs
```

## Модули

### email/ - Email модуль
- `smtp/client.rs` - SMTP клиент
- `smtp/message.rs` - Конструктор писем
- `validator/dns.rs` - DNS валидация
- `validator/smtp_check.rs` - SMTP проверка
- `endorsements/endorsers.rs` - Списки эндорсеров
- `endorsements/sender.rs` - Отправка писем

### Использование

```rust
use cdp_rs::email::{SmtpClient, send_endorsements, get_physicist_endorsers};

let mut smtp = SmtpClient::new("smtp.gmail.com", 587)?;
smtp.auth("user@gmail.com", "pass")?;

let endorsers = get_physicist_endorsers();
let (sent, failed) = send_endorsements(&mut smtp, &endorsers, "from@gmail.com");
```

---
*cdp_rs v4.0 - Модульная архитектура*
