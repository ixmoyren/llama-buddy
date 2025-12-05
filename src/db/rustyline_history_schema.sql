-- 开启一个排他事务
BEGIN EXCLUSIVE;
-- 设置自动清理模式为手动清理
PRAGMA auto_vacuum = INCREMENTAL;
-- 删除原本已有的触发器
DROP TRIGGER IF EXISTS history_bu;
DROP TRIGGER IF EXISTS history_bd;
DROP TRIGGER IF EXISTS history_au;
DROP TRIGGER IF EXISTS history_ai;
-- 删除已有的倒排索引表
DROP TABLE IF EXISTS fts;
-- 创建倒排索引，使用 fts5
CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5
(
    entry,
    content = 'history'
);
-- 为倒排索引创建触发器
CREATE TRIGGER IF NOT EXISTS history_bu
    BEFORE UPDATE
    ON history
BEGIN
    DELETE FROM fts WHERE rowid = old.rowid;
END;
CREATE TRIGGER IF NOT EXISTS history_bd
    BEFORE DELETE
    ON history
BEGIN
    DELETE FROM fts WHERE rowid = old.rowid;
END;
CREATE TRIGGER IF NOT EXISTS history_au
    AFTER UPDATE
    ON history
BEGIN
    INSERT INTO fts (rowid, entry) VALUES (new.rowid, new.entry);
END;
CREATE TRIGGER IF NOT EXISTS history_ai
    AFTER INSERT
    ON history
BEGIN
    INSERT INTO fts (rowid, entry) VALUES (new.rowid, new.entry);
END;
PRAGMA user_version = 2;
COMMIT;