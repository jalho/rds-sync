package main

import (
	"log"
	"net"
	"os"
)

func main() {
	socketPath := "/tmp/asdasd.sock"
	_ = os.Remove(socketPath)
	unixAddr, err := net.ResolveUnixAddr("unixgram", socketPath)
	if err != nil {
		log.Fatal("Error resolving Unix address:", err)
	}

	conn, err := net.ListenUnixgram("unixgram", unixAddr)
	if err != nil {
		log.Fatal("Error listening on Unix socket:", err)
	}
	defer conn.Close()

	buffer := make([]byte, 1024)
	for {
		n, _, err := conn.ReadFromUnix(buffer)
		if err != nil {
			log.Fatal("Error reading from Unix socket:", err)
		}
		log.Printf("Received data: %s\n", buffer[:n])
	}
}

