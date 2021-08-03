limiter, limit your outgoing network traffic when reaching limits
==========================================================

## Background

If you use AWS instances (or any other cloud supplier) you might need to control your costs. A huge amount of costs will be dictated by the outgoing traffic. Imagine your system has failure or an evil person tries to harm you and starts downloading things they can find.

AWS allows you to setup alarms and budgets. Especially the budgets can help you to get informed even via SMS. If you do not plan to pay extra those budgets will not be updated by the minute, not even hour. This means what is causing you to send data to the web will continue.

Moreover, AWS does not give you tools to limit the bandwidth of an instance even when a threshold has been reached. It is still up to the admin to to shutdown things.

This program will automate things for you. You can set how often you want to check how much traffic has been sent on a particular interface. This will also be accounted during restarts of your systems. Once a threshold has been reached the chosen rate will be applied via `tc` (c.f. `man tc`or https://man7.org/linux/man-pages/man8/tc.8.html). If your instance reaches the next threshold it will apply even more limiting.

CAUTION: If you limit the wrong interface with a too low rate you might make your instance inaccessible even via SSH.

`limiter` will reset the limit at the beginning of the month.

## Getting Started

You need to install Rust and compile everything. A simple `cargo build --release` should be sufficient.

Afterwards you can use ([install.sh](install.sh) and go from there. If executed with root priviliges this will create a new user and group and copy the executable, the ([limits.json](limits.json) there. It will also install a system unit which can be controlled via `systemd`: `systemctl start limiter`.
You can verify the successful start via `systemctl status limiter` or even see the logs with `journalctl -xeu limiter`.

## Setting the limits

Just edit ([limits.json](limits.json) to your liking. 
```json
{
    "limit": 200000000000,
    "rate": "12mbit",
    "burst": "48kb",
    "latency": "70ms"
}
```
`limit` should be your preferred limit in bytes, in this example 200 GB. `rate`, `burst`, `latency` accept all values you can use for `tc` (c.f. `man tc`, setion `PARAMETERS` or https://man7.org/linux/man-pages/man8/tc.8.html#PARAMETERS). 


## License

MIT license ([LICENSE](LICENSE) or https://opensource.org/licenses/MIT)

## TODO

* [ ] add `sudo`/`su` to user creation script and in code
* [ ] calculate burst rates
* [ ] improve installation script
* [ ] add better config, not only via environment variables
* [ ] set reset date via config
* [ ] use and combine with `iptables` to limit only specific traffic, c.f. https://www.cyberciti.biz/faq/linux-traffic-shaping-using-tc-to-control-http-traffic/
* [ ] many more things