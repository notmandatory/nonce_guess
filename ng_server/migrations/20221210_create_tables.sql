CREATE TABLE target (block INTEGER, nonce INTEGER WITHOUT ROWID, UNIQUE(block));
CREATE TABLE guess (player_name TEXT NOT NULL, block INTEGER REFERENCES target (block), nonce INTEGER NOT NULL, UNIQUE(player_name, block));
CREATE UNIQUE INDEX idx_guess_null_block ON guess(player_name) WHERE block IS NULL;