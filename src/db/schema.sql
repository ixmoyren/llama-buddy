-- 开启一个排他事务
begin exclusive;
-- 设置自动清理模式为手动清理
pragma auto_vacuum = incremental;

-- 配置表，用来保存一些配置信息
create table if not exists config
(
    id         integer primary key,
    name       text not null unique,
    value      blob not null,
    created_at integer default (strftime('%s', 'now')),
    updated_at integer default (strftime('%s', 'now'))
) strict;

insert into config(name, value)
values ('init_status', cast('Not Started' as blob)),
       ('insert_model_info_completed', cast('Not Started' as blob)),
       ('update_model_info_completed', cast('Not Started' as blob)),
       ('manifest_schema_version', cast(2 as blob)),
       ('manifest_media_type', cast('application/vnd.docker.distribution.manifest.v2+json' as blob)),
       ('model_media_type', cast('application/vnd.ollama.image.model' as blob)),
       ('template_media_type', cast('application/vnd.ollama.image.template' as blob)),
       ('license_media_type', cast('application/vnd.ollama.image.license' as blob)),
       ('params_media_type', cast('application/vnd.ollama.image.params' as blob)),
       ('model', cast('gguf' as blob)),
       ('template', cast('txt' as blob)),
       ('license', cast('txt' as blob)),
       ('params', cast('json' as blob))
on conflict (name) do update set value      = excluded.value,
                                 updated_at = strftime('%s', 'now');

-- 用来保存 ollama.com 模型信息的元数据原始数据
create table if not exists library_raw_data
(
    id         integer primary key,
    href       text not null unique,
    digest     text,
    raw_data   text,
    created_at integer default (strftime('%s', 'now')),
    updated_at integer default (strftime('%s', 'now'))
) strict;

-- 模型信息表
create table if not exists model_info
(
    id           blob primary key,
    title        text not null,
    href         text not null,
    raw_digest   text,
    introduction text,
    pull_count   text,
    tag_count    text,
    summary      text,
    readme       text,
    updated_time text,
    created_at   integer default (strftime('%s', 'now')),
    updated_at   integer default (strftime('%s', 'now'))
) strict;

create unique index if not exists model_info_unique on model_info (title, href);

-- 模型表
create table if not exists model
(
    id         blob primary key,
    name       text not null,
    href       text not null,
    path       text,
    template   text,
    license    text,
    params     text,
    size       text,
    context    text,
    input      text,
    hash       text,
    model_id   blob,
    created_at integer default (strftime('%s', 'now')),
    updated_at integer default (strftime('%s', 'now')),
    foreign key (model_id) references model_info (id)
) strict;

create unique index if not exists model_unique on model (name);

-- 对 title、introduction、summary、readme 创建倒排索引
create virtual table if not exists model_info_fts using fts5
(
    title,
    introduction,
    summary,
    readme,
    content = 'model_info',
    tokenize = 'jieba'
);

create trigger if not exists model_info_after_insert
    after insert
    on model_info
begin
    insert into model_info_fts(title, introduction, summary, readme)
    values (new.title, new.introduction, new.summary, new.readme);
end;

create trigger if not exists model_info_after_delete
    after delete
    on model_info
begin
    insert into model_info_fts(model_info_fts, title, introduction, summary, readme)
    values ('delete', old.title, old.introduction, old.summary, old.readme);
end;

create trigger if not exists model_info_after_update
    after update
    on model_info
begin
    insert into model_info_fts(model_info_fts, title, introduction, summary, readme)
    values ('delete', old.title, old.introduction, old.summary, old.readme);
    insert into model_info_fts(title, introduction, summary, readme)
    values (new.title, new.introduction, new.summary, new.readme);
end;

-- 设置数据库的用户版本好为 1，标识数据库已经初始化
pragma user_version = 1;
commit;
