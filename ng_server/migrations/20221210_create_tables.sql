CREATE TABLE player (uuid TEXT NOT NULL UNIQUE, name TEXT NOT NULL UNIQUE);
CREATE TABLE auth (uuid TEXT NOT NULL REFERENCES player (uuid), passkey BLOB NOT NULL);
CREATE TABLE target (block INTEGER, nonce INTEGER WITHOUT ROWID, UNIQUE(block));
CREATE TABLE guess (uuid TEXT NOT NULL REFERENCES player (uuid), block INTEGER REFERENCES target (block), nonce INTEGER NOT NULL, UNIQUE(uuid, block), UNIQUE(nonce, block));
CREATE UNIQUE INDEX idx_guess_player_name_null_block ON guess(uuid) WHERE block IS NULL;
CREATE UNIQUE INDEX idx_guess_nonce_null_block ON guess(nonce) WHERE block IS NULL;