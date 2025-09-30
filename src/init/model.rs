use crate::db::{Model, ModelInfo};
use anyhow::anyhow;
use http_extra::sha256::digest;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};
use std::collections::VecDeque;
use tracing::debug;
use url::Url;

pub(crate) async fn fetch_library_html(
    client: Client,
    remote_registry: Url,
) -> anyhow::Result<String> {
    let library_url = remote_registry.join("/library?sort=newest")?;
    debug!("Fetching model information from {library_url:?}");
    let response = client.get(library_url).send().await?;
    let library_html = response.text().await?;
    Ok(library_html)
}

// 获取到一个模型的基本信息
// 模型有不同的规格，每个规格的模型一般会提供四个文件，一个是模型本体，一个是许可，一个是模板，一个是提示词
// 通过 href 可以访问到这个模型的详细页面
// 从详细页面中获取模型 summary 和 readme
// 从 /tags 页面可以获取全部的规格列表
pub(crate) async fn fetch_model_more_info(
    model: &ModelInfo,
    client: Client,
    remote_registry: Url,
) -> anyhow::Result<(String, String, String, Vec<Model>)> {
    // 获取模型的 summary 和 readme
    let model_href = model.href.as_str();
    let model_url = remote_registry.join(model_href)?;
    let response = client.get(model_url).send().await?;
    let model_html = response.text().await?;
    let html_str = model_html.as_str();
    let (summary, readme) = convert_to_model_summary(html_str)?;
    // 获取模型的全部 tags
    let model_all_tags_url = format!("{model_href}/tags");
    let model_tags_url = remote_registry.join(model_all_tags_url.as_str())?;
    let response = client.get(model_tags_url).send().await?;
    let model_all_tag_html = response.text().await?;
    let model_tag_vec = covert_to_model_tag(model_all_tag_html)?;
    Ok((summary, readme, model_html, model_tag_vec))
}

fn covert_to_model_tag(html: impl AsRef<str>) -> anyhow::Result<Vec<Model>> {
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

fn convert_to_model_summary(html: impl AsRef<str>) -> anyhow::Result<(String, String)> {
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

pub(crate) fn convert_to_model_infos(html: impl AsRef<str>) -> anyhow::Result<VecDeque<ModelInfo>> {
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

fn get_selector(str: &str) -> anyhow::Result<Selector> {
    Selector::parse(str).map_err(|err| anyhow!("Failed to create the selector, err: {err}"))
}

fn extract_text(el: &ElementRef, selector: &Selector) -> Option<String> {
    el.select(selector)
        .next()
        .map(|el| el.text().collect::<String>())
}
