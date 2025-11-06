use crate::{
    db,
    db::{
        CompletedStatus, Model, ModelInfo, completed_init, insert_model_info,
        save_library_to_library_raw_data,
    },
    error::Whatever,
};
use http_extra::{client, sha256::digest};
use reqwest::Client;
use rusqlite::Connection;
use scraper::{ElementRef, Html, Selector};
use snafu::{FromString, prelude::*};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::Mutex;
use tracing::{debug, error};
use url::Url;

pub(crate) async fn try_save_model_info(
    conn: Arc<Mutex<Connection>>,
    client: Client,
    remote_registry: Url,
) -> Result<(), Whatever> {
    if check_insert_model_info_completed(Arc::clone(&conn)).await? {
        return Ok(());
    }
    match save_model_info(Arc::clone(&conn), client, remote_registry).await {
        Ok(_) => {
            completed_insert_model_info_completed(Arc::clone(&conn), CompletedStatus::Completed)
                .await
        }
        Err(error) => {
            completed_insert_model_info_completed(Arc::clone(&conn), CompletedStatus::Failed)
                .await?;
            Err(error)
        }
    }
}

pub(crate) async fn check_insert_model_info_completed(
    conn: Arc<Mutex<Connection>>,
) -> Result<bool, Whatever> {
    let conn = conn.lock().await;
    db::check_insert_model_info_completed(&conn)
}

pub(crate) async fn completed_insert_model_info_completed(
    conn: Arc<Mutex<Connection>>,
    completed_status: CompletedStatus,
) -> Result<(), Whatever> {
    let conn = conn.lock().await;
    db::completed_insert_model_info_completed(&conn, completed_status)
}

pub(crate) async fn save_model_info(
    conn: Arc<Mutex<Connection>>,
    client: Client,
    remote_registry: Url,
) -> Result<(), Whatever> {
    let old_model_raw_digest_map = query_model_title_and_model_info(Arc::clone(&conn)).await?;
    let (library_html_sender, library_html_receiver) = tokio::sync::oneshot::channel::<String>();
    let (model_info_sender, mut model_info_receiver) = tokio::sync::mpsc::channel(256);
    // 生产者为从 ollama.com 中获取的全部模型列表的数据
    let send_job = tokio::spawn(send(
        client,
        remote_registry,
        library_html_sender,
        model_info_sender,
        old_model_raw_digest_map,
    ));
    let receive_job_one = tokio::spawn(receive_one(Arc::clone(&conn), library_html_receiver));
    let receive_job_two = tokio::spawn(receive_two(Arc::clone(&conn), model_info_receiver));

    match tokio::try_join!(send_job, receive_job_one, receive_job_two) {
        Ok((Ok(_), Ok(_), Ok(_))) => Ok(()),
        Ok((Err(error), _, _)) => Err(Whatever::with_source(
            error.into(),
            "Failed to send library and model info".to_owned(),
        )),
        Ok((_, Err(error), _)) => Err(Whatever::with_source(
            error.into(),
            "Failed to receive library".to_owned(),
        )),
        Ok((_, _, Err(error))) => Err(Whatever::with_source(
            error.into(),
            "Failed to receive model info".to_owned(),
        )),
        Err(error) => Err(Whatever::with_source(
            error.into(),
            "Failed to join all job to tokio".to_owned(),
        )),
    }
}

pub(crate) async fn query_model_title_and_model_info(
    conn: Arc<Mutex<Connection>>,
) -> Result<HashMap<String, String>, Whatever> {
    let conn = conn.lock().await;
    db::query_model_title_and_model_info(&conn)
}
async fn send(
    client: Client,
    remote_registry: Url,
    library_html_sender: tokio::sync::oneshot::Sender<String>,
    model_info_sender: tokio::sync::mpsc::Sender<ModelInfo>,
    old_model_raw_digest_map: HashMap<String, String>,
) -> Result<(), Whatever> {
    let library_html = fetch_library_html(client.clone(), remote_registry.clone()).await?;
    let library_html_str = library_html.as_str();
    let mut model_infos = convert_to_model_infos(library_html_str)?;
    library_html_sender
        .send(library_html)
        .with_whatever_context(|_| "send library html to channel failed!")?;
    for model_info in model_infos.iter_mut() {
        if let Some(old_raw_digest) = old_model_raw_digest_map.get(&model_info.title) {
            if old_raw_digest == model_info.raw_digest.as_str() {
                continue;
            }
        }
        let (summary, readme, html_raw, model_tag_vec) =
            fetch_model_more_info(&model_info, client.clone(), remote_registry.clone()).await?;
        model_info.summary = summary;
        model_info.readme = readme;
        model_info.html_raw = html_raw;
        model_info.models = model_tag_vec;
        model_info_sender
            .send(model_info.to_owned())
            .await
            .with_whatever_context(|_| "send model info to channel failed!")?;
    }
    Ok(())
}

async fn receive_one(
    conn: Arc<Mutex<Connection>>,
    library_html_receiver: tokio::sync::oneshot::Receiver<String>,
) -> Result<(), Whatever> {
    let html = library_html_receiver
        .await
        .with_whatever_context(|_| "receiver one get the library html from channel failed")?;
    let conn = conn.lock().await;
    save_library_to_library_raw_data(&conn, html)?;
    Ok(())
}

async fn receive_two(
    conn: Arc<Mutex<Connection>>,
    mut model_info_receiver: tokio::sync::mpsc::Receiver<ModelInfo>,
) -> Result<(), Whatever> {
    let mut conn = conn.lock().await;
    let mut all_success = true;
    while let Some(model) = model_info_receiver.recv().await {
        if let Ok(is_success) = insert_model_info(&mut conn, model)
            && !is_success
        {
            all_success = false;
        }
    }
    if all_success {
        completed_init(&conn, CompletedStatus::Completed)?;
    } else {
        completed_init(&conn, CompletedStatus::Failed)?;
    }
    Ok(())
}

/// 获取包含全部模型的详情的页面
async fn fetch_library_html(client: Client, remote_registry: Url) -> Result<String, Whatever> {
    let library_url = remote_registry
        .join("/library?sort=newest")
        .with_whatever_context(|_| "Failed to join the library url")?;
    debug!("Fetching model information from {library_url:?}");
    let response = client
        .get(library_url)
        .send()
        .await
        .with_whatever_context(|_| "Failed to fetch the library page")?;
    let library_html = response
        .text()
        .await
        .with_whatever_context(|_| "Failed to read the library page")?;
    Ok(library_html)
}

/// 获取到一个模型的基本信息
///
/// 模型有不同的规格，每个规格的模型一般会提供四个文件，一个是模型本体，一个是许可，一个是模板，一个是提示词
///
/// 通过 href 可以访问到这个模型的详细页面
///
/// 从详细页面中获取模型 summary 和 readme
///
/// 从 /tags 页面可以获取全部的规格列表
async fn fetch_model_more_info(
    model: &ModelInfo,
    client: Client,
    remote_registry: Url,
) -> Result<(String, String, String, Vec<Model>), Whatever> {
    // 获取模型的 summary 和 readme
    let model_href = model.href.as_str();
    let model_url = remote_registry
        .join(model_href)
        .with_whatever_context(|_| "Failed to join the model url")?;
    let response = client
        .get(model_url)
        .send()
        .await
        .with_whatever_context(|_| "Failed to fetch the model page")?;
    let model_html = response
        .text()
        .await
        .with_whatever_context(|_| "Failed to read the model page")?;
    let html_str = model_html.as_str();
    let (summary, readme) = convert_to_model_summary(html_str)
        .with_whatever_context(|_| "Failed to convert the model summary")?;
    // 获取模型的全部 tags
    let model_all_tags_url = format!("{model_href}/tags");
    let model_tags_url = remote_registry
        .join(model_all_tags_url.as_str())
        .with_whatever_context(|_| "Failed to join model tags url")?;
    let response = client
        .get(model_tags_url)
        .send()
        .await
        .with_whatever_context(|_| "Failed to fetch the model tags page")?;
    let model_all_tag_html = response
        .text()
        .await
        .with_whatever_context(|_| "Failed to read the model tags page")?;
    let model_tag_vec = covert_to_model_tag(model_all_tag_html)?;
    Ok((summary, readme, model_html, model_tag_vec))
}

fn covert_to_model_tag(html: impl AsRef<str>) -> Result<Vec<Model>, Whatever> {
    let html = Html::parse_document(html.as_ref());
    let tag_table = get_selector("body section > div > div > div")?;
    let tag_href = get_selector("div > span > a")?;
    let tag_p = get_selector("div > p")?;
    let tag_input = get_selector("div > div.col-span-2")?;
    let tag_hash = get_selector("div >div >span.font-mono")?;
    let mut models = Vec::<Model>::new();
    for x in html.select(&tag_table) {
        let Some(href_el) = x.select(&tag_href).next() else {
            continue;
        };
        let Some(input_el) = x.select(&tag_input).next() else {
            continue;
        };
        let mut tag_p_select = x.select(&tag_p);
        let Some(size_el) = tag_p_select.next() else {
            continue;
        };
        let Some(context_el) = tag_p_select.next() else {
            continue;
        };
        let Some(hash_el) = x.select(&tag_hash).next() else {
            continue;
        };
        let name = href_el.inner_html();
        let href = if let Some(href) = href_el.attr("href") {
            href.to_owned()
        } else {
            "".to_owned()
        };
        let size = size_el.inner_html();
        let context = context_el.inner_html();
        let input = input_el.inner_html();
        let hash = hash_el.inner_html();
        let model = Model {
            name,
            href,
            size,
            context,
            input,
            hash,
            ..Default::default()
        };
        models.push(model);
    }
    Ok(models)
}

fn convert_to_model_summary(html: impl AsRef<str>) -> Result<(String, String), Whatever> {
    let html = Html::parse_document(html.as_ref());
    let summary = get_selector("#summary-content")?;
    let readme = get_selector("#readme #display")?;
    let summary = html
        .select(&summary)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or("".to_owned());
    let readme = html
        .select(&readme)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or("".to_owned());
    Ok((summary, readme))
}

/// 将模型详细信息页转换成 VecDeque<ModelInfo>
fn convert_to_model_infos(html: impl AsRef<str>) -> Result<VecDeque<ModelInfo>, Whatever> {
    let html = Html::parse_document(html.as_ref());
    let li_selector = get_selector("div#repo > ul li a")?;
    let title_selector = get_selector("div [x-test-model-title]")?;
    let introduction_selector = get_selector("p")?;
    let pull_count_selector = get_selector("span [x-test-pull-count]")?;
    let tag_count_selector = get_selector("span [x-test-tag-count]")?;
    let updated_time_selector = get_selector("span [x-test-updated]")?;
    let mut models = VecDeque::<ModelInfo>::new();

    for el in html.select(&li_selector) {
        let el_html = el.html();
        let raw_digest = if el_html == "" {
            "".to_owned()
        } else {
            digest(el.html().as_bytes())
        };
        let href = if let Some(href) = el.attr("href") {
            href.to_owned()
        } else {
            "".to_owned()
        };
        let Some(title_el) = el.select(&title_selector).next() else {
            continue;
        };
        let Some(title) = title_el.attr("title") else {
            continue;
        };
        let introduction = extract_text(&title_el, &introduction_selector);
        let pull_count = extract_text(&el, &pull_count_selector);
        let tag_count = extract_text(&el, &tag_count_selector);
        let updated_time = extract_text(&el, &updated_time_selector);
        let (Some(introduction), Some(pull_count), Some(tag_count), Some(updated_time)) =
            (introduction, pull_count, tag_count, updated_time)
        else {
            continue;
        };
        let model_info = ModelInfo {
            title: title.to_owned(),
            href,
            raw_digest,
            introduction,
            pull_count,
            tag_count,
            updated_time,
            ..Default::default()
        };
        models.push_front(model_info);
    }
    Ok(models)
}

fn get_selector(selector_str: &'static str) -> Result<Selector, Whatever> {
    Selector::parse(selector_str).map_err(|error| {
        error!("{error:?}");
        Whatever::without_source(format!("Failed to get selector from {selector_str}"))
    })
}

fn extract_text(el: &ElementRef, selector: &Selector) -> Option<String> {
    el.select(selector)
        .next()
        .map(|el| el.text().collect::<String>())
}
