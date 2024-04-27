package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"time"
)

/*
Logger shenanigans to make it emit ISO formatted timestamps.
*/
type PrefixedLogOutput struct {
	write_prefix func() string
	writer       io.Writer
}

func (self PrefixedLogOutput) Write(payload []byte) (n int, err error) {
	// write a prefix
	prefix := []byte(self.write_prefix())
	bytes_written_prefix, err_write_prefix := self.writer.Write(prefix)
	if err_write_prefix != nil {
		return
	}
	// write the actual payload
	bytes_written_payload, err_write_payload := self.writer.Write(payload)
	return bytes_written_prefix + bytes_written_payload, err_write_payload
}

func alert_discord(webhook_url string, webhook_message_content string) {
	webhook_payload_structured := map[string]string{
		"content": webhook_message_content,
	}
	webhook_payload_serialized, err_webhook_payload_serialize := json.Marshal(webhook_payload_structured)
	if err_webhook_payload_serialize != nil {
		log.Printf("Error while serializing Discord webhook payload: %v", err_webhook_payload_serialize)
		return
	}
	webhook_post_response, err_webhook_post := http_post(webhook_url, webhook_payload_serialized)
	if err_webhook_post != nil {
		log.Printf("Error while doing HTTP POST to Discord webhook: %v", err_webhook_post)
		return
	}
	defer webhook_post_response.Body.Close()
	log.Printf("Discord webhook POST response status: %v", webhook_post_response.Status)
}

func http_post(url string, payload []byte) (*http.Response, error) {
	req, err := http.NewRequest("POST", url, bytes.NewBuffer(payload))
	if err != nil {
		return nil, fmt.Errorf("Error creating HTTP request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Error sending HTTP request: %v", err)
	}
	return resp, nil
}

const (
	PvP Category = iota
	PvE
	Farm
	World
)

type Category int
type ActivityMessage struct {
	Category   Category `json:"category"`
	Timestamp  uint64   `json:"timestamp"`
	ID_Subject string   `json:"id_subject"`
	ID_Object  string   `json:"id_object"`
	Quantity   uint64   `json:"quantity"`
}

type Stat struct {
	Quantity        uint
	TimestampInit   uint64
	TimestampLatest uint64
}

func handle_message(event ActivityMessage, store map[string]map[string]Stat, webhook_url string) {
	switch event.Category {
	case PvP:
		log.Printf(
			"TODO: Got a 'PvP' event! %s -> %s",
			event.ID_Subject, event.ID_Object)
	case PvE:
		log.Printf(
			"TODO: Got a 'PvE' event! %s -> %s",
			event.ID_Subject, event.ID_Object)
	case Farm:
		log.Printf(
			"Got a 'Farm' event! %s -> %s: %d",
			event.ID_Subject, event.ID_Object, event.Quantity)
		accumulate_stats(store, event.ID_Subject, event.ID_Object, uint(event.Quantity), event.Timestamp)
		stat := get_stat(store, event.ID_Subject, event.ID_Object)
		log.Printf(
			"'Farm' stats accumulated! %s -> %s: total: %d (from %s to %s)",
			event.ID_Subject, event.ID_Object, stat.Quantity, as_date_iso(stat.TimestampInit), as_date_iso(stat.TimestampLatest),
		)
	case World:
		switch event.ID_Subject {
		case "OnCargoShipSpawnCrate":
			log.Printf("Alerting Discord!")
			alert_discord(webhook_url, "Crate spawned on Cargo Ship!")
		}
	default:
		// Nothing to see here!
	}
}

func as_date_iso(timestamp uint64) string {
	t := time.Unix(int64(timestamp), 0)
	return t.Format(time.RFC3339)
}

func accumulate_stats(store map[string]map[string]Stat, id_subject string, id_object string, quantity uint, timestamp uint64) {
	if _, ok := store[id_subject]; !ok {
		store[id_subject] = make(map[string]Stat)
	}

	if _, ok := store[id_subject][id_object]; !ok {
		store[id_subject][id_object] = Stat{
			Quantity:        quantity,
			TimestampInit:   timestamp,
			TimestampLatest: timestamp,
		}
	} else {
		store[id_subject][id_object] = Stat{
			Quantity:        quantity + store[id_subject][id_object].Quantity,
			TimestampInit:   store[id_subject][id_object].TimestampInit,
			TimestampLatest: timestamp,
		}
	}
}

func get_stat(store map[string]map[string]Stat, id_subject string, id_object string) Stat {
	if _, ok := store[id_subject]; ok {
		if stat, ok := store[id_subject][id_object]; ok {
			return stat
		}
	}
	// TODO: return some kinda error/null thingy instead?
	return Stat{}
}

/*
WHAT DO?

This program receives messages about game events over a Unix domain socket.
The sender is another process on the same host (namely a Carbon plugin loaded
into RustDedicated).
*/
func main() {
	// set up logger
	log.SetFlags(0)
	log_writer := PrefixedLogOutput{
		write_prefix: func() string { return "[" + time.Now().Format(time.RFC3339) + "] " },
		writer:       log.Writer(),
	}
	log.SetOutput(log_writer)

	/*
	  Stats accumulated in memory.
	*/
	store_inmem := make(map[string]map[string]Stat)

	// set up stats receiving socket
	socket_fs_path := "/tmp/rds-stats-collector.sock"
	_ = os.Remove(socket_fs_path)
	socket_unix_addr, err := net.ResolveUnixAddr("unixgram", socket_fs_path)
	if err != nil {
		log.Fatal("Error resolving Unix address:", err)
	}
	conn, err := net.ListenUnixgram("unixgram", socket_unix_addr)
	if err != nil {
		log.Fatal("Error listening on Unix socket:", err)
	}
	defer conn.Close()

	// get messages and do stuff about them...
	webhook_url := "http://127.0.0.1:8080/api/webhooks/0000000000000000000/0aa0aaaaaaa0aaaaa0aaaaaaaaaaaaaa0aaaa0aaaaaaa_aa-aaaaa_0aaaaaaaaa0aa"
	buffer_inbound := make([]byte, 1024)
	for {
		// receive a message
		n, _, err_read_inbound := conn.ReadFromUnix(buffer_inbound)
		if err_read_inbound != nil {
			log.Fatal("Error reading from Unix socket:", err_read_inbound)
		}
		message_inbound := string(buffer_inbound[:n])

		// parse the received message
		var activity_message_structured ActivityMessage
		err_activity_message_unmarshal := json.Unmarshal([]byte(message_inbound), &activity_message_structured)
		if err_activity_message_unmarshal != nil {
			log.Printf("Error while unmarshalling inbound message: %v", err_activity_message_unmarshal)
			continue
		}
		log.Printf("Got message: timestamp: %d, category: %d, subject: '%s', object: '%s'",
			activity_message_structured.Timestamp, activity_message_structured.Category,
			activity_message_structured.ID_Subject, activity_message_structured.ID_Object)
		handle_message(activity_message_structured, store_inmem, webhook_url)
	}
}
