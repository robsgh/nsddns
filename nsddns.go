package main

import (
	"encoding/json"
	"encoding/xml"
	"errors"
	"flag"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"path/filepath"
)

/* GLOBAL VARS */

// Namesilo API key
var NS_API_KEY string

// Namesilo API version
var NS_VERSION_NAME string = "1"

// Namesilo format querystring for getting DNS records
var NS_DNS_GET_URL string = "https://www.namesilo.com/api/dnsListRecords?version=" +
	NS_VERSION_NAME + "&type=xml&key=%s&domain=%s"

// Namesilo format querystring for updating DNS record
var NS_DNS_UPDATE_URL string = "https://www.namesilo.com/api/dnsUpdateRecord?version=" +
	NS_VERSION_NAME + "&type=xml&key=%s&domain=%s&rrid=%s&rrhost=%s&rrvalue=%s"

/* BEGIN NAMESILO API XML STRUCTS */

type NamesiloGet struct {
	XMLName xml.Name         `xml:"namesilo"`
	Request NamesiloRequest  `xml:"request"`
	Reply   NamesiloGetReply `xml:"reply"`
}

type NamesiloUpdate struct {
	XMLName xml.Name            `xml:"namesilo"`
	Request NamesiloRequest     `xml:"request"`
	Reply   NamesiloUpdateReply `xml:"reply"`
}

type NamesiloRequest struct {
	Operation string `xml:"operation"`
	Ip        string `xml:"ip"`
}

type NamesiloGetReply struct {
	Code            int                      `xml:"code"`
	Detail          string                   `xml:"detail"`
	ResourceRecords []NamesiloResourceRecord `xml:"resource_record"`
}

type NamesiloUpdateReply struct {
	Code     int    `xml:"code"`
	Detail   string `xml:"detail"`
	RecordId string `xml:"record_id"`
}

type NamesiloResourceRecord struct {
	Id       string `xml:"record_id"`
	Type     string `xml:"type"`
	Host     string `xml:"host"`
	Value    string `xml:"value"`
	TTL      int    `xml:"ttl"`
	Distance int    `xml:"distance"`
}

/* END NAMESILO API XML STRUCTS */

// Application configuration
type NSDDNSConfig struct {
	Domain string
	Host   string
	ApiKey string
}

// Parse the XML data from Namesilo API and return a list of the domain's RRs
func parseNamesiloDNSRecords(xmlBytes []byte) (rrs []NamesiloResourceRecord, err error) {
	// process the XML response
	ns := NamesiloGet{}
	err = xml.Unmarshal(xmlBytes, &ns)
	if err != nil {
		err = errors.New("error unmarshalling xml data: " + err.Error())
		return
	}

	// NS API returns 300 if the request was successful
	if ns.Reply.Code != 300 {
		errString := fmt.Sprintf("namesilo API reply code was not 300: %s\n", ns.Reply.Detail)
		err = errors.New(errString)
		return
	}

	// return the list of resource records
	rrs = ns.Reply.ResourceRecords
	return
}

// Get the DNS A record from Namesilo using the API
func getDNSFromNamesilo(domain string, host string) (rr NamesiloResourceRecord, err error) {
	// populate format string URL with params and fire off GET req
	url := fmt.Sprintf(NS_DNS_GET_URL, NS_API_KEY, domain)
	resp, err := http.Get(url)
	if err != nil {
		return
	}
	respXMLBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return
	}

	// parse the XML data and get a list of RRs
	rrs, err := parseNamesiloDNSRecords(respXMLBytes)
	// find the DNS A RR which is desired
	for _, item := range rrs {
		// only process A records
		if item.Type != "A" {
			continue
		}

		// check if host value matches
		if item.Host == host {
			rr = item
			return
		}
	}

	// reaching end of loop implies there is no host RR that matches desired
	err = errors.New("no matching host resource record: " + host + " for apex domain " + domain)
	return
}

// Set the DNS A record value of a domain to a specific IP
func setNamesiloDNSIP(domain string, host string, rr NamesiloResourceRecord, ip string) (id string, err error) {
	// populate the url params and fire off get req
	apiurl := fmt.Sprintf(NS_DNS_UPDATE_URL, NS_API_KEY, domain, rr.Id, host, ip)
	resp, err := http.Get(apiurl)
	if err != nil {
		return
	}
	xmlBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return
	}

	// decode the XML data into the NamesiloUpdate struct
	ns := NamesiloUpdate{}
	err = xml.Unmarshal(xmlBytes, &ns)
	if err != nil {
		return
	}

	// validate the NS API response was successful (300 is success in NS API)
	if ns.Reply.Code != 300 {
		errString := fmt.Sprintf("namesilo API reply code was not 300: %s\n", ns.Reply.Detail)
		err = errors.New(errString)
		return
	}

	// return newly-updated record ID on success (is probably different)
	return ns.Reply.RecordId, nil
}

// Get the current IP of the executing machine through ipify.org
func getCurrentIP() (currentIP string) {
	// use ipify to get the IP of the executing machine
	resp, err := http.Get("https://api.ipify.org")
	if err != nil {
		log.Fatalln("error getting IP from api.ipify.org:", err)
	}

	// body of ipify message is: ^IP$
	ipBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		log.Fatalln("error reading IP response body:", err)
	}

	// thus, returning the entire body is fine
	currentIP = string(ipBytes)
	return
}

// Load a configuration file from conf.json in the same directory
func loadConfig() NSDDNSConfig {
	var configPath string

	// load the conf.json file that should be in the same dir as the executable
	execPath, err := os.Executable()
	if err != nil {
		log.Fatalln("could not determine current working dir:", err.Error())
	}
	exeDir := filepath.Dir(execPath)

	// attempt to parse the --config flag
	flag.StringVar(&configPath, "config", exeDir+"/conf.json", "Set the config file path")
	flag.Parse()

	// use to configPath var to open the file conf
	configFile, err := os.Open(configPath)
	if err != nil {
		log.Fatalln("could not open conf.json:", err.Error())
	}
	defer configFile.Close()

	// decode it as json
	config := NSDDNSConfig{}
	decoder := json.NewDecoder(configFile)
	err = decoder.Decode(&config)
	if err != nil {
		log.Fatalln("could not decode json conf.json:", err.Error())
	}

	return config
}

func main() {
	var fullHostname string

	// load the user config
	config := loadConfig()

	// set the variables from the config
	domain := config.Domain
	host := config.Host
	if host == "" {
		fullHostname = config.Domain
	} else {
		fullHostname = config.Host + "." + config.Domain
	}
	NS_API_KEY = config.ApiKey

	// get current IP and the DNS A record IP
	ip := getCurrentIP()
	dnsIp, err := getDNSFromNamesilo(domain, fullHostname)
	if err != nil {
		log.Fatalln("error while getting DNS from Namesilo:", err)
	}

	// update A record if necessary
	log.Printf("Current IP: %s\tRR (id: %s): %s\tNeeds update: %t\n", ip, dnsIp.Id, dnsIp.Value, ip != dnsIp.Value)
	if dnsIp.Value != ip {
		rrId, err := setNamesiloDNSIP(domain, host, dnsIp, ip)
		if err != nil {
			log.Fatalln("error setting namesilo IP:", err)
		}

		log.Println("Namesilo DNS RR", rrId, "updated to", ip)
	}

}
