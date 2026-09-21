-- Structural rollback only: recreates the shared counter's absence. The
-- sequence/trigger delta layer is NOT restored (its trigger bodies are in
-- 000005); a rollback past this point should run 000005 again if needed.
DROP TABLE IF EXISTS contacts.change_counter;
