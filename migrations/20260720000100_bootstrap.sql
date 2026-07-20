CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE platform_metadata (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    initialized_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO platform_metadata (singleton)
VALUES (TRUE)
ON CONFLICT (singleton) DO NOTHING;

