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
    fn execute(&self, url: &Url, html: &str) -> Result<JsResult> {
        let scripts = collect_scripts(html, url);
        if scripts.is_empty() {
            return Ok(JsResult {
                html: None,
                console: Vec::new(),
                final_url: None,
            });
        }

        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;

        ctx.with(|ctx| {
            let shim = concat!(
                "var window=globalThis;var self=globalThis;",
                "var console={log:function(){},warn:function(){},error:function(){},info:function(){},debug:function(){}};",
                "var document={readyState:'complete',cookie:'',title:'',",
                "  getElementById:function(){return null;},",
                "  querySelector:function(){return null;},",
                "  querySelectorAll:function(){return[];},",
                "  getElementsByTagName:function(){return[];},",
                "  getElementsByClassName:function(){return[];},",
                "  getElementsByName:function(){return[];},",
                "  createElement:function(t){return{tagName:t.toUpperCase(),style:{},classList:{add:function(){},remove:function(){},contains:function(){return false;},toggle:function(){}},dataset:{},children:[],childNodes:[],parentNode:null,setAttribute:function(){},getAttribute:function(){return null;},hasAttribute:function(){return false;},removeAttribute:function(){},appendChild:function(c){return c;},removeChild:function(c){return c;},insertBefore:function(c){return c;},addEventListener:function(){},removeEventListener:function(){},dispatchEvent:function(){return true;},innerHTML:'',innerText:'',textContent:'',value:'',checked:false,disabled:false,type:'',name:'',href:'',src:''};},",
                "  createTextNode:function(t){return{nodeValue:t,textContent:t};},",
                "  createDocumentFragment:function(){return{appendChild:function(){},childNodes:[]};},",
                "  body:{appendChild:function(){return arguments[0];},removeChild:function(){},insertBefore:function(){return arguments[0];},style:{},classList:{add:function(){},remove:function(){},contains:function(){return false;}},children:[],childNodes:[],innerHTML:'',dataset:{},addEventListener:function(){},querySelector:function(){return null;},querySelectorAll:function(){return[];}},",
                "  head:{appendChild:function(){return arguments[0];},querySelector:function(){return null;},querySelectorAll:function(){return[];}},",
                "  documentElement:{style:{},lang:'en',dir:'ltr',setAttribute:function(){},classList:{add:function(){},remove:function(){},contains:function(){return false;}}},",
                "  addEventListener:function(){},removeEventListener:function(){},dispatchEvent:function(){return true;},",
                "  write:function(){},writeln:function(){},",
                "  cookie:'',referrer:'',",
                "  implementation:{hasFeature:function(){return true;}}",
                "};",
                "var navigator={userAgent:'Mozilla/5.0 clibrowser/0.1',language:'en-US',languages:['en-US','en'],platform:'Linux',onLine:true,cookieEnabled:false,doNotTrack:'1'};",
                "var screen={width:1920,height:1080,availWidth:1920,availHeight:1080,colorDepth:24,pixelDepth:24};",
                "var location={href:'',origin:'',pathname:'/',search:'',hash:'',hostname:'',protocol:'https:',port:'',assign:function(){},replace:function(){},reload:function(){}};",
                "var history={length:1,state:null,pushState:function(){},replaceState:function(){},back:function(){},forward:function(){},go:function(){}};",
                "var localStorage={_d:{},getItem:function(k){return this._d[k]||null;},setItem:function(k,v){this._d[k]=String(v);},removeItem:function(k){delete this._d[k];},clear:function(){this._d={};},get length(){return Object.keys(this._d).length;}};",
                "var sessionStorage={_d:{},getItem:function(k){return this._d[k]||null;},setItem:function(k,v){this._d[k]=String(v);},removeItem:function(k){delete this._d[k];},clear:function(){this._d={};},get length(){return Object.keys(this._d).length;}};",
                "var performance={now:function(){return 0;},timing:{},mark:function(){},measure:function(){},getEntriesByType:function(){return[];}};",
                "var MutationObserver=function(cb){this.observe=function(){};this.disconnect=function(){};this.takeRecords=function(){return[];};};",
                "var IntersectionObserver=function(cb,opts){this.observe=function(){};this.unobserve=function(){};this.disconnect=function(){};};",
                "var ResizeObserver=function(cb){this.observe=function(){};this.unobserve=function(){};this.disconnect=function(){};};",
                "var CustomEvent=function(type,init){this.type=type;this.detail=init&&init.detail||null;this.bubbles=false;this.cancelable=false;};",
                "var Event=function(type,init){this.type=type;this.bubbles=(init&&init.bubbles)||false;this.cancelable=(init&&init.cancelable)||false;this.target=null;this.preventDefault=function(){};this.stopPropagation=function(){};};",
                "var XMLHttpRequest=function(){this.readyState=0;this.status=0;this.responseText='';this.response=null;this.onreadystatechange=null;this.onload=null;this.onerror=null;this.open=function(){};this.send=function(){};this.setRequestHeader=function(){};this.getResponseHeader=function(){return null;};this.addEventListener=function(){};this.abort=function(){};};",
                "function fetch(url,opts){return new Promise(function(resolve){resolve({ok:true,status:200,headers:{get:function(){return null;},has:function(){return false;}},json:function(){return Promise.resolve({});},text:function(){return Promise.resolve('');},blob:function(){return Promise.resolve(new Blob());},clone:function(){return this;}});});}",
                "var Blob=function(parts,opts){this.size=0;this.type=(opts&&opts.type)||'';this.text=function(){return Promise.resolve('');};this.arrayBuffer=function(){return Promise.resolve(new ArrayBuffer(0));};};",
                "var FormData=function(){this._d=[];this.append=function(k,v){this._d.push([k,v]);};this.get=function(k){var r=this._d.find(function(p){return p[0]===k;});return r?r[1]:null;};this.has=function(k){return this._d.some(function(p){return p[0]===k;});};};",
                "function setTimeout(fn,ms){try{if(typeof fn==='function')fn();}catch(e){}return 0;}",
                "function setInterval(fn,ms){return 0;}",
                "function clearTimeout(id){}function clearInterval(id){}",
                "function requestAnimationFrame(fn){return 0;}function cancelAnimationFrame(id){}",
                "function queueMicrotask(fn){try{if(typeof fn==='function')fn();}catch(e){}}",
                // Commonly used polyfills/utils
                "if(typeof Symbol==='undefined'){var Symbol=function(desc){return '__sym_'+(desc||'')+'_'+Math.random().toString(36).slice(2);};Symbol.iterator='__sym_iterator';Symbol.toPrimitive='__sym_toprimitive';}",
                "if(typeof WeakMap==='undefined'){var WeakMap=Map;}",
                "if(typeof WeakSet==='undefined'){var WeakSet=Set;}",
                "if(typeof WeakRef==='undefined'){var WeakRef=function(t){this.deref=function(){return t;};};}"
            );

            let _: rquickjs::Result<rquickjs::Value<'_>> = ctx.eval(shim);

            for script in &scripts {
                if let Err(e) = ctx.eval::<rquickjs::Value<'_>, _>(script.code.as_str()) {
                    tracing::debug!("QuickJS [{}]: {:?}", script.src, e);
                }
            }
        });

        Ok(JsResult {
            html: None,
            console: Vec::new(),
            final_url: None,
        })
    }

    fn name(&self) -> &'static str {
        "quickjs"
    }
}

struct Script {
    src: String,
    code: String,
}

/// Collect all scripts (inline and external) from an HTML document in order.
fn collect_scripts(html: &str, base_url: &Url) -> Vec<Script> {
    let mut scripts = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0;

    while let Some(rel) = lower[pos..].find("<script") {
        let tag_start = pos + rel;
        let _after_open = &html[tag_start..];
        let lower_after = &lower[tag_start..];

        let tag_close = match lower_after.find('>') {
            Some(i) => i,
            None => break,
        };

        let attrs_str = &lower_after[7..tag_close]; // skip "<script"

        // Skip non-JS types
        if let Some(type_val) = extract_attr_value(attrs_str, "type") {
            if !type_val.contains("javascript") && type_val != "text/ecmascript" && !type_val.is_empty() {
                pos = tag_start + tag_close + 1;
                continue;
            }
        }

        // Skip module scripts for now (they need ES module resolution)
        if attrs_str.contains(r#"type="module""#) || attrs_str.contains("type='module'") {
            pos = tag_start + tag_close + 1;
            continue;
        }

        if let Some(src) = extract_attr_value(attrs_str, "src") {
            // External script — fetch it
            if let Some(url) = base_url.join(&src).ok() {
                let url_str = url.to_string();
                match fetch_external_script(&url_str) {
                    Ok(code) => {
                        tracing::debug!("Fetched external script: {}", url_str);
                        scripts.push(Script { src: url_str, code });
                    }
                    Err(e) => {
                        tracing::debug!("Failed to fetch {}: {}", url_str, e);
                    }
                }
            }
            // Skip to close tag (no inline body in src= scripts)
            let rest = &lower[tag_start + tag_close + 1..];
            let skip = rest.find("</script>").unwrap_or(rest.len());
            pos = tag_start + tag_close + 1 + skip + 9;
        } else {
            // Inline script
            let content = &html[tag_start + tag_close + 1..];
            let lower_content = &lower[tag_start + tag_close + 1..];
            if let Some(end) = lower_content.find("</script>") {
                let code = content[..end].trim().to_string();
                if !code.is_empty() {
                    scripts.push(Script {
                        src: "(inline)".to_string(),
                        code,
                    });
                }
                pos = tag_start + tag_close + 1 + end + 9;
            } else {
                break;
            }
        }
    }

    scripts
}

fn extract_attr_value(attrs: &str, name: &str) -> Option<String> {
    // Find name= in attrs string (lowercased)
    let search = format!("{}=", name);
    let idx = attrs.find(search.as_str())?;
    let rest = &attrs[idx + name.len() + 1..];
    let value = if rest.starts_with('"') {
        rest[1..].split('"').next()?.to_string()
    } else if rest.starts_with('\'') {
        rest[1..].split('\'').next()?.to_string()
    } else {
        rest.split_whitespace().next()?.to_string()
    };
    Some(value)
}

fn fetch_external_script(url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("clibrowser/0.1")
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let resp = client.get(url).send()?;
    if resp.status().is_success() {
        Ok(resp.text()?)
    } else {
        anyhow::bail!("HTTP {}", resp.status())
    }
}
