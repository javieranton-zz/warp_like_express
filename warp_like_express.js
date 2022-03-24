const {warp_like_express} = require("./warp_like_express.node");
const {warp_like_express_response_callback} = require("./warp_like_express.node");
let getEndpoints = [];
let getCallbacks = new Map();
module.exports.get = function(path, request){
    if (path.startsWith('/'))
         path = path.substring(1);
    getEndpoints.push(path);
    getCallbacks.set(path, request);
};
let onErrorCallback = (err) =>{
};
let getCallBack = (endpoint, queryStringsString, requestHeaders, uuid) => {
    console.log(requestHeaders);
    let requestHeadersObject = JSON.parse(requestHeaders);
    let request = {query:{}, header: (name) => {
        return requestHeadersObject[name] ? requestHeadersObject[name] : "unknown";
    }};
    let queryStrings = queryStringsString.split("&");
    for(const queryString of queryStrings){
        let queryStringTuple = queryString.split('=');
        request.query[queryStringTuple[0]] = queryStringTuple[1];
    }
    //console.log("Rs -> Js: "+endpoint);
    let resHeaders = {};
    let res = {send: (data)=>{
        if (data === undefined)
            data = "";
        if(data instanceof Buffer)
            resHeaders["content-type"] = "*/*";
        else if(typeof data === 'object' || Array.isArray(data)){
             resHeaders["content-type"] = "application/json";
             data = Buffer.from(JSON.stringify(data),"utf-8");
        }
        else{
             resHeaders["content-type"] = "text/html";
             data = Buffer.from(data,"utf-8");
        }
        let args = [uuid,JSON.stringify(resHeaders),data];
        dispatch(warp_like_express_response_callback,args);
    }, header: (name, value)=>{
        resHeaders[name] = value;
    }}
    getCallbacks.get(endpoint)(request,res);
};
module.exports.listen = function(port, callBack){
    let config = { port : port, getEndpoints: getEndpoints.join(',')};
    let args = [JSON.stringify(config)];
    //get callback
    args.push(getCallBack);
    //done callback
    args.push(callBack);
    //error
    args.push(onErrorCallback);
    dispatch(warp_like_express,args);
    return this;
};
module.exports.on = function(event, callBack){
    switch(event){
        case "error":
            onErrorCallback = callBack;
            break;
    }
    return this;
}
//call a function with an indeterminate number of arguments
function dispatch(fn, args) {
    return fn.apply(this, args || []);  // args is optional, use an empty array by default
}
