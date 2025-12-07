package main

import (
	"crypto/rsa"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"

	"github.com/golang-jwt/jwt/v5"
)

type KVHandlers struct {
	kv        map[string]map[string]string
	publicKey *rsa.PublicKey
}

func NewKVHandlers(publicKeyFile string) *KVHandlers {
	data, err := os.ReadFile(publicKeyFile)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	publicKey, err := jwt.ParseRSAPublicKeyFromPEM(data)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	return &KVHandlers{
		kv:        make(map[string]map[string]string),
		publicKey: publicKey,
	}
}

func (h *KVHandlers) put(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "put can be done only with POST HTTP method")
		return
	}
	queryParams := req.URL.Query()
	if !queryParams.Has("key") || len(queryParams["key"]) != 1 {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "there is no or too many keys in query params")
		return
	}
	key := queryParams["key"][0]
	body := make([]byte, req.ContentLength)
	read, err := req.Body.Read(body)
	defer req.Body.Close()
	if read != int(req.ContentLength) {
		w.WriteHeader(http.StatusBadRequest)
		return
	}
	if err != io.EOF {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "Error reading body: %v", err)
		return
	}
	bodyJson := make(map[string]string)
	err = json.Unmarshal(body, &bodyJson)
	value, ok := bodyJson["value"]
	if err != nil || !ok {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "Error unmarshalling body: %v", err)
		return
	}
	if _, ok = h.kv[key]; !ok {
		h.kv[key] = make(map[string]string)
	}
	h.kv[key]["value"] = value
}

func (h *KVHandlers) get(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodGet && req.Method != http.MethodPost {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "get can be done only with GET or POST HTTP method")
		return
	}
	queryParams := req.URL.Query()
	if !queryParams.Has("key") || len(queryParams["key"]) != 1 {
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "there is no or too many keys in query params")
		return
	}
	key := queryParams["key"][0]
	kvData, ok := h.kv[key]
	if !ok {
		w.WriteHeader(http.StatusNotFound)
		fmt.Fprintf(w, "could not find key %v in storage", key)
		return
	}

	value, ok := kvData["value"]
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "kv data does not have 'value' key somehow")
		return
	}
	resp := make(map[string]string)
	resp["value"] = value
	respJson, err := json.Marshal(resp)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "Error marshalling response: %v", err)
		return
	}
	w.Header().Add("Content-Type", "application/json")
	_, err = w.Write(respJson)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, "Error writing response: %v", err)
		return
	}
}

func main() {
	publicKeyFile := flag.String("public", "", "path to public key file")
	port := flag.Int("port", 8091, "HTTP server port")
	flag.Parse()

	if port == nil {
		fmt.Fprintln(os.Stderr, "Port is required")
		os.Exit(1)
	}

	if publicKeyFile == nil || *publicKeyFile == "" {
		fmt.Fprintln(os.Stderr, "Please provide a path to public key file")
		os.Exit(1)
	}

	absPublicKeyFile, err := filepath.Abs(*publicKeyFile)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	kv := NewKVHandlers(absPublicKeyFile)

	http.HandleFunc("/put", kv.put)
	http.HandleFunc("/get", kv.get)

	fmt.Println("Starting server on port", *port, "with public key file", absPublicKeyFile)

	if err = http.ListenAndServe(fmt.Sprintf(":%d", *port), nil); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
