//@todo document this in the readme.
//@todo list all currently banned IPs/Miners.
//@todo list all current miners and their difficulty, etc.
//@todo launch a separate API for metrics

// @todo let's probably move API to it's own folder so we can spread things out more.

//The <> here is state, so add to it as we need it.

//@todo 2 checks we want here.
//1. Basic healthcheck, literally just an http endoint /ping that returns 200
//2. More complex that checks all dependent services are working. That might be able to be saved by
//   whatever is implementing this above, but we can brainstorm more on this one.
