# nsddns - Namesilo Dynamic DNS
Cron-able tool for updating Namesilo DNS A record value to current public IP, like DDNS.

## Configuration

Configuration is done through the `conf.json` file. As the name implies, it is a JSON-formatted file. The following keys must be set:

* `domain`: domain name which is registered through Namesilo
* `host`: hostname of the DNS A record (can be empty, in which case the A record would be the domain)
* `apikey`: Namesilo API key which can be generated through the Namesilo API manager portal

The `conf.json` file must live in the same directory as the binary.

### Example `conf.json`

```json
{
    "domain": "example.com",
    "host": "test",
    "apikey": "1234abcd"
}
```

## Usage

Running `./nsddns` will grab the user settings supplied in `conf.json` and start the automation. There are no flags currently; all user configuration
is done through conf.json.

## Building nsddns

Build the go project by running `go build` in the project directory. This will create an executable called `nsddns`. Additionally,
`go install` can be used to place the binary at the `$GOPATH/bin` location. If `$GOPATH/bin` is added to your `PATH` environment variable
then `nsddns` will be able to be used anywhere in the shell.

## Recommended Usage

Copy the `nsddns` executable to a directory which only `root` can access. Copy `conf.json` to this directory with file perms `0600` and owner `root:root`.
Then, add a cronjob every couple of minutes which executes `nsddns`.
