# cdp_rs - Modular Browser Automation Framework

## For User

Модульная библиотека для автоматизации браузера, отправки email и управления endorsement на arXiv.

## For LLM

### Бинарник

`E:\1\cdp_rs\target\release\cdp_rs.exe`

### Модули

```
cdp_rs/
├── email/                    # 📧 Email модуль
│   ├── smtp/                 # SMTP клиент
│   ├── validator/            # Валидация email
│   └── endorsements/         # Endorsement система
├── browser/                  # 🌐 Browser модуль
└── page/                     # 📄 Page модуль
```

### Паттерн 1: Отправка email

```rust
use cdp_rs::email::smtp::SmtpClient;

let mut smtp = SmtpClient::new("smtp.gmail.com", 587)?;
smtp.auth("user@gmail.com", "app_password")?;
smtp.send_email("from@gmail.com", "to@example.com", "Subject", "Body")?;
```

### Паттерн 2: Валидация email

```rust
use cdp_rs::email::validator::dns::validate_email;

let result = validate_email("user@example.com");
println!("{}", result); // ✓ VALID: MX records found
```

### Паттерн 3: Endorsement emails

```rust
use cdp_rs::email::endorsements::{get_physicist_endorsers, send_endorsements};
use cdp_rs::email::smtp::SmtpClient;

let mut smtp = SmtpClient::new("smtp.gmail.com", 587)?;
smtp.auth("user@gmail.com", "app_password")?;

let endorsers = get_physicist_endorsers();
let (sent, failed) = send_endorsements(&mut smtp, &endorsers, "from@gmail.com");
```

### Паттерн 4: Browser automation

```rust
use cdp_rs::browser::BrowserManager;
use cdp_rs::page::Page;

let browser = BrowserManager::new().launch().await?;
let page = Page::new(browser.connection().unwrap());
page.navigate("https://example.com").await?;
page.screenshot("screenshot.png").await?;
```

### Команды

```
cdp_rs page <url>              # Открыть страницу
cdp_rs screenshot <url>        # Скриншот
cdp_rs pdf <url>               # PDF
send_physicists                # Отправка endorsement
```

### Структура модулей

```
email/
├── smtp/
│   ├── client.rs              # SMTP клиент
│   └── message.rs             # Конструктор писем
├── validator/
│   ├── dns.rs                 # DNS валидация
│   └── smtp_check.rs          # SMTP проверка
└── endorsements/
    ├── endorsers.rs           # Списки эндорсеров
    └── sender.rs              # Отправка писем
```

### Зависимости

- native-tls - TLS
- base64 - Кодирование
- tokio - Async runtime

---
*cdp_rs v4.0 - Модульная архитектура*
