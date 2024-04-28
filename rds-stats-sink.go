package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

/*
Logger shenanigans to make it emit ISO formatted timestamps.
*/
type PrefixedLogOutput struct {
	write_prefix func() string
	writer       io.Writer
}

func (log_output PrefixedLogOutput) Write(payload []byte) (n int, err error) {
	// write a prefix
	prefix := []byte(log_output.write_prefix())
	bytes_written_prefix, err_write_prefix := log_output.writer.Write(prefix)
	if err_write_prefix != nil {
		return
	}
	// write the actual payload
	bytes_written_payload, err_write_payload := log_output.writer.Write(payload)
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
		return nil, fmt.Errorf("error creating HTTP request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("error sending HTTP request: %v", err)
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
	Category          Category `json:"category"`
	Timestamp_unix_ms uint64   `json:"timestamp"`
	ID_Subject        string   `json:"id_subject"`
	ID_Object         string   `json:"id_object"`
	Quantity          uint64   `json:"quantity"`
}

type Stat struct {
	Quantity                 uint
	Timestamp_unix_ms_init   uint64
	Timestamp_unix_ms_latest uint64
}

func handle_message(event ActivityMessage, store map[string]map[string]Stat, webhook_url_alert_cargoship string) {
	switch event.Category {
	case PvP:
		log.Printf(
			"TODO: Got a PvP event! %s -> %s",
			event.ID_Subject, event.ID_Object)
	case PvE:
		log.Printf(
			"TODO: Got a PvE event! %s -> %s",
			event.ID_Subject, event.ID_Object)
	case Farm:
		accumulate_stats(store, event.ID_Subject, event.ID_Object, uint(event.Quantity), event.Timestamp_unix_ms)
		stat := get_stat(store, event.ID_Subject, event.ID_Object)
		log.Printf(
			"Farm stats accumulated! %s -> %s: total: %d (from %s to %s)",
			event.ID_Subject, event.ID_Object, stat.Quantity, as_date_iso(stat.Timestamp_unix_ms_init), as_date_iso(stat.Timestamp_unix_ms_latest),
		)
	case World:
		switch event.ID_Subject {
		case "OnCargoShipSpawnCrate":
			log.Printf("Alerting Discord!")
			alert_discord(webhook_url_alert_cargoship, "Crate spawned on Cargo Ship!")
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
			Quantity:                 quantity,
			Timestamp_unix_ms_init:   timestamp,
			Timestamp_unix_ms_latest: timestamp,
		}
	} else {
		store[id_subject][id_object] = Stat{
			Quantity:                 quantity + store[id_subject][id_object].Quantity,
			Timestamp_unix_ms_init:   store[id_subject][id_object].Timestamp_unix_ms_init,
			Timestamp_unix_ms_latest: timestamp,
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

func receive_events_from_rds_plugin_over_unix_sock(store_inmem map[string]map[string]Stat, webhook_url_alert_cargoship string, wg *sync.WaitGroup) {
	defer wg.Done()

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
		handle_message(activity_message_structured, store_inmem, webhook_url_alert_cargoship)
	}
}

var upgrader = websocket.Upgrader{} // use default options

func handle_websocket(w http.ResponseWriter, r *http.Request) {
	socket, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Print("Error while upgrading HTTP to WebSocket:", err)
		return
	}
	defer socket.Close()
	for {
		message_type_inbound, message_inbound, err := socket.ReadMessage()
		if err != nil {
			log.Println("Error while reading a message from WebSocket:", err)
			break
		}
		message_outbound := message_inbound
		err = socket.WriteMessage(message_type_inbound, message_outbound)
		if err != nil {
			log.Println("Error while writing an echo message to WebSocket:", err)
			break
		}

		store_json, err := json.Marshal(store_inmem)
		if err != nil {
			log.Println("Error marshalling store to JSON:", err)
			return
		}
		// TODO: define message type
		err = socket.WriteMessage(message_type_inbound, store_json)
		if err != nil {
			log.Println("Error while writing a message to WebSocket:", err)
			break
		}
	}
}

/*
Stats accumulated in memory.
*/
var store_inmem = make(map[string]map[string]Stat)

/*
WHAT DO?

This program receives messages about game events over a Unix domain socket.
The sender is another process on the same host (namely a Carbon plugin loaded
into RustDedicated).
*/
func main() {
	var webhook_url_alert_cargoship string
	flag.StringVar(&webhook_url_alert_cargoship, "alert-cargoship", "", "Discord web hook URL for Cargo Ship alerts")
	var http_listen_addr string
	flag.StringVar(&http_listen_addr, "http-listen-addr", "0.0.0.0:8080", "HTTP/WebSocket service address")
	flag.Parse()

	// set up logger
	log.SetFlags(0)
	log_writer := PrefixedLogOutput{
		write_prefix: func() string { return "[" + time.Now().Format(time.RFC3339) + "] " },
		writer:       log.Writer(),
	}
	log.SetOutput(log_writer)

	var wg sync.WaitGroup

	wg.Add(1)
	go receive_events_from_rds_plugin_over_unix_sock(store_inmem, webhook_url_alert_cargoship, &wg)

	http.HandleFunc("/", handle_websocket)
	log.Fatal(http.ListenAndServe(http_listen_addr, nil))

	wg.Wait()
}
