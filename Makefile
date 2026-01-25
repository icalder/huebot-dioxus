IMAGE_NAME ?= icalder/huebot
VERSION ?= latest
LOCAL_NAME = huebot

.PHONY: all build load manifest push clean release

all: build

build:
	nix build .#docker-all

load: build
	podman load < result/huebot-x86_64.tar.gz
	podman load < result/huebot-aarch64.tar.gz

manifest: load
	-podman manifest rm $(IMAGE_NAME):$(VERSION) 2>/dev/null || true
	podman manifest create $(IMAGE_NAME):$(VERSION)
	podman manifest add $(IMAGE_NAME):$(VERSION) containers-storage:localhost/$(LOCAL_NAME):x86_64
	podman manifest add $(IMAGE_NAME):$(VERSION) containers-storage:localhost/$(LOCAL_NAME):arm64

push: manifest
	podman manifest push $(IMAGE_NAME):$(VERSION) docker://docker.io/$(IMAGE_NAME):$(VERSION)

clean:
	-podman manifest rm $(IMAGE_NAME):$(VERSION) 2>/dev/null || true
	-podman rmi localhost/$(LOCAL_NAME):x86_64 2>/dev/null || true
	-podman rmi localhost/$(LOCAL_NAME):aarch64 2>/dev/null || true
	-podman rmi localhost/$(LOCAL_NAME):arm64 2>/dev/null || true
	rm -rf result

release: clean push