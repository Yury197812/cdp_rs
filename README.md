# Orchestrator LLM

Система для массовой обработки задач в ChatGPT с управлением проектами.

## Быстрый старт

### Docker

```bash
# Сборка и запуск
docker-compose up -d

# Проверка
curl http://localhost:8080/health
```

### Локальная установка

```bash
# Клонирование
git clone https://github.com/Yury197812/cdp_rs.git
cd cdp_rs

# Сборка
cargo build --release

# Запуск
./target/release/orchestrator
```

## Конфигурация

Скопируйте `config.example.toml` в `config.toml` и отредактируйте:

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
path = "data/orchestrator.db"
```

## API

```
GET  /api/overview          - Обзор системы
GET  /api/projects          - Список проектов
POST /api/projects          - Создать проект
GET  /api/projects/:id      - Детали проекта
GET  /api/projects/:id/branches - Ветки проекта
GET  /api/projects/:id/tasks    - Задачи проекта
```

## Архитектура

```
orchestrator/
├── core/           # Ядро системы
├── workers/        # Рабочие узлы
├── coordinator/    # Координация
└── dashboard/      # Веб-интерфейс
```

## Лицензия

MIT
