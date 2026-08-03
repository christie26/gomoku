IMAGE_NAME := localhost/omuk-gomoku-toolbox-img:latest
BOX_NAME   := omuk-gomoku-toolbox

.PHONY: build run enter clean re

run: build
	@if ! podman container exists $(BOX_NAME); then \
		toolbox create --image $(IMAGE_NAME) $(BOX_NAME); \
	fi
	toolbox enter $(BOX_NAME)

enter:
	toolbox enter $(BOX_NAME)

build:
	podman build -t $(IMAGE_NAME) .

clean:
	-podman container exists $(BOX_NAME) && toolbox rm -f $(BOX_NAME)
	-podman image exists $(IMAGE_NAME) && podman rmi -f $(IMAGE_NAME)

re: clean run
