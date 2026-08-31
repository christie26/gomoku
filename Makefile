TOOLBOX_NAME = nix-toolbox
IMAGE_NAME = nix-toolbox-image

.PHONY: create-image create-box shell destroy reset

# Build the custom image
create-image: Containerfile
	@podman build -t $(IMAGE_NAME) -f Containerfile .

# Create the container if it doesn't exist
create-box: create-image
	@podman container exists $(TOOLBOX_NAME) || toolbox create --image $(IMAGE_NAME) $(TOOLBOX_NAME)

# Launch the Nix environment
shell: create-box
	@toolbox run -c $(TOOLBOX_NAME) env \
		XDG_CACHE_HOME=/tmp/.cache \
		XDG_DATA_HOME=/tmp/.local/share \
		XDG_STATE_HOME=/tmp/.local/state \
		nix develop

# Wipe container and restart cleanly
reset: destroy shell

# Remove old container
destroy:
	@toolbox rm -f $(TOOLBOX_NAME) || true
