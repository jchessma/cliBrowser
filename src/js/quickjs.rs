use anyhow::Result;
use rquickjs::{Context, Runtime};
use url::Url;

use super::engine::{JsEngine, JsResult};

pub struct QuickJsEngine;

impl QuickJsEngine {
    pub fn new() -> Self {
        Self
    }
}

impl JsEngine for QuickJsEngine {
    fn execute(&self, _url: &Url, html: &str) -> Result<JsResult> {
        let scripts = extract_inline_scripts(html);
        if scripts.is_empty() {
            return Ok(JsResult {
                html: None,
                console: Vec::new(),
            });
        }

        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;

        ctx.with(|ctx| {
            // Minimal browser environment shims (single-line to avoid escape issues)
            let shim = concat!(
                "var window=globalThis;",
                "var self=globalThis;",
                "var console={log:function(){},warn:function(){},error:function(){},info:function(){}};",
                "var document={readyState:'complete',cookie:'',title:'',",
                "  getElementById:function(){return null;},",
                "  querySelector:function(){return null;},",
                "  querySelectorAll:function(){return[];},",
                "  getElementsByTagName:function(){return[];},",
                "  getElementsByClassName:function(){return[];},",
                "  createElement:function(t){return{tagName:t,style:{},classList:{add:function(){},remove:function(){},contains:function(){return false;}},setAttribute:function(){},getAttribute:function(){return null;},appendChild:function(){},removeChild:function(){}};},",
                "  createTextNode:function(t){return{nodeValue:t};},",
                "  body:{appendChild:function(){},removeChild:function(){},style:{},classList:{add:function(){}}},",
                "  head:{appendChild:function(){}},",
                "  addEventListener:function(){},",
                "  removeEventListener:function(){},",
                "  dispatchEvent:function(){return true;}",
                "};",
                "var navigator={userAgent:'clibrowser/0.1',language:'en-US',languages:['en-US','en'],platform:'Linux',onLine:true};",
                "var location={href:'',pathname:'/',search:'',hash:'',hostname:'',protocol:'https:',port:''};",
                "var history={pushState:function(){},replaceState:function(){},back:function(){},forward:function(){}};",
                "var localStorage={getItem:function(){return null;},setItem:function(){},removeItem:function(){},clear:function(){},length:0};",
                "var sessionStorage={getItem:function(){return null;},setItem:function(){},removeItem:function(){},clear:function(){},length:0};",
                "function setTimeout(fn,ms){return 0;}",
                "function setInterval(fn,ms){return 0;}",
                "function clearTimeout(id){}",
                "function clearInterval(id){}",
                "function requestAnimationFrame(fn){return 0;}",
                "function cancelAnimationFrame(id){}",
                "function fetch(){return Promise.resolve({ok:true,json:function(){return Promise.resolve({});},text:function(){return Promise.resolve('');},headers:{get:function(){return null;}}});}",
                "var XMLHttpRequest=function(){this.open=function(){};this.send=function(){};this.setRequestHeader=function(){};this.addEventListener=function(){};};",
            );

            let _: rquickjs::Result<rquickjs::Value<'_>> = ctx.eval(shim);

            for script in &scripts {
                if let Err(e) = ctx.eval::<rquickjs::Value<'_>, _>(script.as_str()) {
                    tracing::debug!("QuickJS script error: {:?}", e);
                }
            }
        });

        Ok(JsResult {
            html: None,
            console: Vec::new(),
        })
    }

    fn name(&self) -> &'static str {
        "quickjs"
    }
}

fn extract_inline_scripts(html: &str) -> Vec<String> {
    let mut scripts = Vec::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;

    while let Some(tag_start) = lower[search_from..].find("<script") {
        let abs_tag_start = search_from + tag_start;
        let _after_tag = &html[abs_tag_start..];
        let lower_after = &lower[abs_tag_start..];

        let tag_end = match lower_after.find('>') {
            Some(i) => i,
            None => break,
        };

        let tag_attrs = &lower_after[..tag_end];
        // Skip external scripts (src attribute)
        if tag_attrs.contains("src=") {
            search_from = abs_tag_start + tag_end + 1;
            continue;
        }
        // Skip type=module and non-JS types
        if tag_attrs.contains("type=") && !tag_attrs.contains("javascript") {
            search_from = abs_tag_start + tag_end + 1;
            continue;
        }

        let content = &html[abs_tag_start + tag_end + 1..];
        let lower_content = &lower[abs_tag_start + tag_end + 1..];

        if let Some(close_pos) = lower_content.find("</script>") {
            let script = content[..close_pos].trim();
            if !script.is_empty() {
                scripts.push(script.to_string());
            }
            search_from = abs_tag_start + tag_end + 1 + close_pos + 9;
        } else {
            break;
        }
    }

    scripts
}
