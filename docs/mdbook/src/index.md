# Observer

Observer is a lightweight Rust agent that collects metrics from your machine and stores them 
locally. You can connect our iOS application to your client to visualize different host metrics.
Currently, we are working on a web-interface which you will be able to selfhost and use either 
alongside the iOS applicaiton or standalone.

For Questions or our community please join our [Discord](https://discord.gg/Xnh7nKdsnM) server. 
You are more than welcome, and we always search for interested people.

## What it monitors

- CPU, RAM, disk, and network usage
- System info (hostname, OS, uptime, IP)
- Docker container CPU and memory
- Network speed (download, upload, latency)
- Component health state changes

## How it works

Observer runs as a background service, periodically collecting metrics and POSTing them to
a backend endpoint. It authenticates with an API key and can poll the backend for 
remote commands to execute.

## This is a early access version

As this is an early access version you application on you server shouldn't fail. It is more likely
that our *Watch-Tower* backend fails. There is a mechanism which shuts down you backend if it cant connect after x amount of retries to our server. If you dont get metrics in the frontend. This is a likely cause. So please go ahead and just restart observer on your server.

```sh
systemctl restart observer
```
