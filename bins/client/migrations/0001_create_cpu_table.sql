CREATE TABLE IF NOT EXISTS cpu (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cpu_name TEXT NOT NULL,
    cpu_count INTEGER NOT NULL,
    cpu_physical_count INTEGER NOT NULL,
    cpu_usage_percent REAL NOT NULL,
    cpu_temperature_celsius REAL NOT NULL,
    recorded_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cpu_core (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cpu_id INTEGER NOT NULL REFERENCES cpu(id),
    core_name TEXT NOT NULL,
    core_usage_percent REAL NOT NULL,
    core_frequency_mhz INTEGER NOT NULL
);
