use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::str::FromStr;
use std::sync::{Arc, Mutex, Once};
use neon::context::{Context, ModuleContext, FunctionContext};
use neon::types::{JsBuffer, JsString};
use neon::types::JsFunction;
use neon::types::JsUndefined;
use neon::result::NeonResult;
use neon::result::JsResult;
use neon::event::Channel;
use neon::handle::Handle;
use neon::object::Object;
use warp::{Filter};
use warp::http::{HeaderMap, HeaderValue, Response};
use lazy_static::lazy_static;
use std::sync::mpsc::{channel};
use neon::types::buffer::TypedArray;
use uuid::Uuid;
use warp::filters::BoxedFilter;
use warp::http::header::HeaderName;
use warp::hyper::Body;

lazy_static! {
    static ref CALLBACK: Arc<Mutex<Option<neon::handle::Root<neon::types::JsFunction>>>> =
        Arc::new(Mutex::new(None));
    static ref Q: Arc<Mutex<Option<Channel>>> = Arc::new(Mutex::new(None));
}

struct Databridge {
    port_u_16: Mutex<u16>,
    //contains a dynamic list of waiting channels to be replied to from the js... so responses can be sent
    wait_for_js_callback_signal : Mutex<HashMap<String, std::sync::mpsc::Sender<()>>>,
    wait_for_js_callback_headers : Mutex<HashMap<String, String>>,
    wait_for_js_callback_body : Mutex<HashMap<String, Vec<u8>>>,
}


fn singleton() -> &'static Databridge {
    // Create an uninitialized static
    static mut SINGLETON: MaybeUninit<Databridge> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = Databridge {
                port_u_16: Mutex::new(0),
                wait_for_js_callback_signal: Mutex::new(HashMap::new()),
                wait_for_js_callback_headers: Mutex::new(HashMap::new()),
                wait_for_js_callback_body: Mutex::new(HashMap::new()),
            };
            // Store it to the static var, i.e. initialize it
            SINGLETON.write(singleton);
        });

        // Now we give out a shared reference to the data, which is safe to use
        // concurrently.
        SINGLETON.assume_init_ref()
    }
}
#[tokio::main]
async fn warp_like_express_api(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let config = cx.argument::<JsString>(0)?.value(&mut cx);
    let mut json_config = json::parse(&config).unwrap();
    let port : f64 = match json_config["port"] {
        json::JsonValue::Number(n) => n.into(),
        _ => panic!("Unexpected port value"),
    };
    let mut port_u_16 = singleton().port_u_16.lock().unwrap();
    *port_u_16 = port as u16;
    let get_endpoints_string : String = json_config["getEndpoints"].take_string().unwrap();
    let get_paths :Vec<String> = get_endpoints_string.split(",").map(|s| s.to_string()).collect();
    CALLBACK
        .lock()
        .unwrap()
        .replace(cx.argument::<JsFunction>(1)?.root(&mut cx));
    //Set global ref to queue
    Q.lock().unwrap().replace(cx.channel());
    let done_callback : Handle<JsFunction> = cx.argument::<JsFunction>(2)?;
    let _x : Handle<JsUndefined> = done_callback
        .call_with(&mut cx)
        .apply(&mut cx)?;
    let error_callback : Handle<JsFunction> = cx.argument::<JsFunction>(3)?;
    let _x : Handle<JsUndefined> = error_callback
        .call_with(&mut cx)
        .arg(cx.string("This is a test error"))
        .apply(&mut cx)?;
    std::thread::spawn(|| {
        start_server(get_paths);
    });
    Ok(cx.undefined())
}
//let tokio choose number of worker threads, which will be 1/core/hyperthread. We could give it more, but then we run the risk of having parallel operations that max out mem (like parsing large strings)
//you can do this if your server has large memory and many cores, otherwise keep it simple
//#[tokio::main(core_threads = 8)]
#[tokio::main]
async fn start_server(get_paths : Vec<String>){
    let mut get_routes:Option<BoxedFilter<_>> = None;
    let get_paths_iter: _ = get_paths.iter();
    for val in get_paths_iter{
        let val_cloned = val.clone();
        let new_get = warp::path(val.to_owned()).and(warp::query()).and(warp::header::headers_cloned()).and_then(move|query_params: Vec<(String, String)>, headers: HeaderMap|{
            let val_cloned_cloned = val_cloned.clone();
            async move{
                get_hit(val_cloned_cloned, query_params, headers).and_then(|response| {
                    Ok(response)
                })
                    .or_else(|_e| Err(warp::reject::reject()))
            }
        });
        get_routes = match get_routes{
            Some(not_null) => Option::Some(not_null.or(new_get).unify().boxed()),
            None =>  Option::Some(new_get.boxed())
        };

    }
    let port_u_16 = singleton().port_u_16.lock().unwrap();
    let get_routes_clone = get_routes.clone();
    if let Some(not_null_routes) = get_routes_clone{
        warp::serve(not_null_routes)
            .run(([0, 0, 0, 0], *port_u_16))
            .await;
    }
}

fn get_callback_js(path:String, query_strings : String, headers_json: String, uuid : String) {
    if let Some(ref queue) = *Q.lock().unwrap() {
        queue.send(|mut cx| {
            if let Some(ref root_callback) = *CALLBACK.lock().unwrap() {
                let callback = root_callback.to_inner(&mut cx);
                let this = cx.undefined();
                let args = vec![cx.string(path).upcast(), cx.string(query_strings).upcast(), cx.string(headers_json).upcast(), cx.string(uuid).upcast()];
                callback.call(&mut cx, this, args)?;
            }
            Ok(())
        }).join().unwrap();
    }
}

fn get_hit(path : String, query_params : Vec<(String, String)>, headers: HeaderMap) -> Result<impl warp::Reply, warp::Rejection> {
    let mut final_query_strings = String::new();
    let query_params_iter: _ = query_params.iter();
    let query_params_iter_len = query_params_iter.len();
    let mut query_params_iter_n = 0;
    for val in query_params_iter{
        final_query_strings = final_query_strings + &val.0 + "=" + &val.1;
        if query_params_iter_n + 1 != query_params_iter_len {
            final_query_strings = final_query_strings + "&";
        }
        query_params_iter_n = query_params_iter_n + 1;
    }
    // Serialize it to a JSON string.
    let mut headers_map: HashMap<String,String> = HashMap::new();
    let headers_iter = headers.iter();
    for val in headers_iter{
        headers_map.insert(val.0.to_string(), String::from(val.1.to_str().unwrap()));
    }
    let json_headers = serde_json::to_string(&headers_map).unwrap();

    let (tx,rx) = channel();
    let new_uuid = Uuid::new_v4().to_string();
    singleton().wait_for_js_callback_signal.lock().unwrap().insert(new_uuid.clone(),tx as std::sync::mpsc::Sender<()>);
    get_callback_js(path, final_query_strings, json_headers, new_uuid.clone());
    rx.recv().expect("Could not receive from channel.");
    // Continue working
    //this is inferred from lib code in warp json reply
    let mut res :Response<Body>= Response::new((singleton().wait_for_js_callback_body.lock().unwrap().get(&new_uuid.clone()).unwrap().to_vec()).into());
    let headers = res.headers_mut();
    let header_hashmap: HashMap<String, String> = serde_json::from_str(singleton().wait_for_js_callback_headers.lock().unwrap().get(&new_uuid.clone()).unwrap()).unwrap();
    for (key, value) in header_hashmap.into_iter() {
        headers.insert(HeaderName::from_str(&key).unwrap(), HeaderValue::from_str(&value).unwrap());
    }
    singleton().wait_for_js_callback_body.lock().unwrap().remove(&new_uuid.clone());
    singleton().wait_for_js_callback_headers.lock().unwrap().remove(&new_uuid.clone());
    Ok(res)
}
#[tokio::main]
async fn warp_like_express_response_callback_api(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let uuid = cx.argument::<JsString>(0)?.value(&mut cx);
    let headers_string = cx.argument::<JsString>(1)?.value(&mut cx);
    //println!("reply headers string: {}", headers_string.clone());
    let body_js_bytes: Handle<JsBuffer> = cx.argument(2)?;
    let body_bytes = body_js_bytes.as_slice(&mut cx).to_vec();
    if singleton().wait_for_js_callback_signal.lock().unwrap().contains_key(&uuid) {
        singleton().wait_for_js_callback_headers.lock().unwrap().insert(uuid.clone(),headers_string);
        singleton().wait_for_js_callback_body.lock().unwrap().insert(uuid.clone(),body_bytes);
        singleton().wait_for_js_callback_signal.lock().unwrap().get(&uuid).unwrap().send(()).expect("Could not send signal on channel.");
        singleton().wait_for_js_callback_signal.lock().unwrap().remove(&uuid);
    }
    //println!("Received Js -> Rs send signal for call {}", uuid.clone());
    Ok(cx.undefined())
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("warp_like_express", warp_like_express_api)?;
    cx.export_function("warp_like_express_response_callback", warp_like_express_response_callback_api)?;
    Ok(())
}