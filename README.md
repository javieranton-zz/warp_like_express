# This is an incomplete Rust replacement for ExpressJS

It runs as a native compiled rust library within your Node.js project. Its aim is to make it easy to switch from using ExpressJS to Warp (Rust) while keeping a Node.js runtime and using a similar syntax

I initially coded this with the intention of completely finishing the project. While performance tests show that response times are at least 30% faster and both memory and CPU usages are also 30% lower, I ran into the realization that it isn't suited for my project in mind. I now think that it's better to run the Rust-Warp server completely on its own, only using Node.js for the initial call that starts the Rust server instead of using JS code for the endpoint backend logic. 30% performance gain isn't enough for me to justify finishing this project. Using a pure-rust backend logic produces incredible results - depending on the # of cores, it can be in the order of 2,000% performance gain. At the end of the day, JS backend logic will go through a Node.js bottleneck no matter how efficient the web server sitting in front of it is

But I did spend a fair bit of time coding this and I think that it might help others if I share it. So here it is

## Current state

You can open GET endpoints that include request query strings and headers. Paths must be absolute (no slashes yet). The endpoints can reply with any type of data (text,html,json, or any buffer) and can also set its own custom headers

No POST/other functionality is available, but it can be developed. Feel free to fork/PR

# Compiling

You will need to compile using the same target environment. For example, if you plan on using this on Google App Engine's Node.js Standard Environment, you will need to compile from a 2020 Linux distro to match GAE's glib.c runtime. The compiled binary included in this release is exactly that

To compile, simply run

```console
npm install
cargo build
npm run build
```

This will produce the `warp_like_express.node` file that you can then import into your Node.js project

# Testing

The file `warp_like_express_test.js` is included to test opening a Rust server from this project. To run, simply run

```console
node warp_like_express_test.js
```

This will start a rust-warp server listening on port 8081 with the following endpoints:
```console
GET /test_text
GET /test_json
GET /test_data
GET /test_image
```
Each endpoint is served by a rust-warp server that then calls your JS backend to execute your custom logic

# Usage

To use in your project, you need to copy both the `warp_like_express.node` and `warp_like_express.js` into your Node directory

The following code opens the above 4 endpoints, and includes ExpressJS's equivalent in comments
```javascript
var rust_or_express = require("./warp_like_express.js");
// equivalent to using below
// const rust_or_express = require('express');

rust_or_express.get("test_text",(req, res)=>{
    console.log("test_text called with query strings: "+JSON.stringify(req.query));
    res.send("hi from js");
});
rust_or_express.get("test_json",(req, res)=>{
    console.log("test_json called with query strings: "+JSON.stringify(req.query));
    res.send({"some":"data"});
});
rust_or_express.get("test_data",(req, res)=>{
    var array = new Buffer.from('TWFuIGlzIGRpc3Rpbmd1aXNoZWQsIG5vdCBvbmx5IGJ5IGhpcyByZWFzb24sIGJ1dCBieSB0aGlzIHNpbmd1bGFyIHBhc3Npb24gZnJvbSBvdGhlciBhbmltYWxzLCB3aGljaCBpcyBhIGx1c3Qgb2YgdGhlIG1pbmQsIHRoYXQgYnkgYSBwZXJzZXZlcmFuY2Ugb2YgZGVsaWdodCBpbiB0aGUgY29udGludWVkIGFuZCBpbmRlZmF0aWdhYmxlIGdlbmVyYXRpb24gb2Yga25vd2xlZGdlLCBleGNlZWRzIHRoZSBzaG9ydCB2ZWhlbWVuY2Ugb2YgYW55IGNhcm5hbCBwbGVhc3VyZS4=', 'base64');
    console.log("test_data called with query strings: "+JSON.stringify(req.query));
    res.send(array);
});
rust_or_express.get("test_image",(req, res)=>{
    var array = new Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABgAAAAYCAYAAADgdz34AAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAAApgAAAKYB3X3/OAAAABl0RVh0U29mdHdhcmUAd3d3Lmlua3NjYXBlLm9yZ5vuPBoAAANCSURBVEiJtZZPbBtFFMZ/M7ubXdtdb1xSFyeilBapySVU8h8OoFaooFSqiihIVIpQBKci6KEg9Q6H9kovIHoCIVQJJCKE1ENFjnAgcaSGC6rEnxBwA04Tx43t2FnvDAfjkNibxgHxnWb2e/u992bee7tCa00YFsffekFY+nUzFtjW0LrvjRXrCDIAaPLlW0nHL0SsZtVoaF98mLrx3pdhOqLtYPHChahZcYYO7KvPFxvRl5XPp1sN3adWiD1ZAqD6XYK1b/dvE5IWryTt2udLFedwc1+9kLp+vbbpoDh+6TklxBeAi9TL0taeWpdmZzQDry0AcO+jQ12RyohqqoYoo8RDwJrU+qXkjWtfi8Xxt58BdQuwQs9qC/afLwCw8tnQbqYAPsgxE1S6F3EAIXux2oQFKm0ihMsOF71dHYx+f3NND68ghCu1YIoePPQN1pGRABkJ6Bus96CutRZMydTl+TvuiRW1m3n0eDl0vRPcEysqdXn+jsQPsrHMquGeXEaY4Yk4wxWcY5V/9scqOMOVUFthatyTy8QyqwZ+kDURKoMWxNKr2EeqVKcTNOajqKoBgOE28U4tdQl5p5bwCw7BWquaZSzAPlwjlithJtp3pTImSqQRrb2Z8PHGigD4RZuNX6JYj6wj7O4TFLbCO/Mn/m8R+h6rYSUb3ekokRY6f/YukArN979jcW+V/S8g0eT/N3VN3kTqWbQ428m9/8k0P/1aIhF36PccEl6EhOcAUCrXKZXXWS3XKd2vc/TRBG9O5ELC17MmWubD2nKhUKZa26Ba2+D3P+4/MNCFwg59oWVeYhkzgN/JDR8deKBoD7Y+ljEjGZ0sosXVTvbc6RHirr2reNy1OXd6pJsQ+gqjk8VWFYmHrwBzW/n+uMPFiRwHB2I7ih8ciHFxIkd/3Omk5tCDV1t+2nNu5sxxpDFNx+huNhVT3/zMDz8usXC3ddaHBj1GHj/As08fwTS7Kt1HBTmyN29vdwAw+/wbwLVOJ3uAD1wi/dUH7Qei66PfyuRj4Ik9is+hglfbkbfR3cnZm7chlUWLdwmprtCohX4HUtlOcQjLYCu+fzGJH2QRKvP3UNz8bWk1qMxjGTOMThZ3kvgLI5AzFfo379UAAAAASUVORK5CYII=','base64');
    console.log("test_image called with query strings: "+JSON.stringify(req.query));
    res.send(array);
});
rust_or_express.listen(8081, (result) =>{
    console.log("Server listening on port 8081");
}).on('error', function(err){
   console.log(err);
});
```
