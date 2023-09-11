# nsddns - Namesilo Dynamic DNS
Cron-able tool for updating Namesilo DNS A record value to current public IP, like DDNS.

## Configuration

Configuration is done through the `conf.json` file. As the name implies, it is a JSON-formatted file. The following keys must be set:

* `domain`: domain name which is registered through Namesilo
* `host`: hostname of the DNS A record (can be empty, in which case the A record would be the domain)
* `apikey`: Namesilo API key which can be generated through the Namesilo API manager portal

The `conf.json` file must live in the same directory as the binary. Alternatively, the `--config` flag can be used to
direct `nsddns` to an alternative JSON configuration file.

### Example `conf.json`

```json
{
    "domain": "example.com",
    "host": "test",
    "apikey": "1234abcd"
}
```

### Example of using `conf.otherdomain.json` and `--config`

`$ ./nsddns --config /path/to/other/dir/conf.otherdomain.json`

## Usage

Running `./nsddns` will grab the user settings supplied in `conf.json` and start the automation.

## Building nsddns

`nsddns` uses Cargo, so you can build the project with `cargo build`. The output binary will be in `bin/`.

## Recommended Usage

1. Copy the `nsddns` executable to `/usr/bin/nsddns`
2. Copy `conf.json` to `/etc/nsddns/conf.json`
3. Copy the files under `systemd/` to `/etc/systemd/system/` to setup the service and timer
4. Run `systemctl enable --now nsddns.timer` to enable and start the timer
