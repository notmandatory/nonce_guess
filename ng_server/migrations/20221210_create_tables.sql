CREATE TABLE target (block INTEGER, nonce INTEGER WITHOUT ROWID, UNIQUE(block));
CREATE TABLE guess (player_name TEXT NOT NULL, block INTEGER REFERENCES target (block), nonce INTEGER NOT NULL, UNIQUE(player_name, block), UNIQUE(nonce, block));
CREATE UNIQUE INDEX idx_guess_player_name_null_block ON guess(player_name) WHERE block IS NULL;
CREATE UNIQUE INDEX idx_guess_nonce_null_block ON guess(nonce) WHERE block IS NULL;