-- Your SQL goes here
ALTER TABLE messages
ADD COLUMN user_id INTEGER REFERENCES users(id) NOT NULL;
