################################################################################
#
# musicos-init
#
################################################################################

MUSICOS_INIT_SITE = $(BR2_EXTERNAL_MUSICOS_PATH)/..
MUSICOS_INIT_SITE_METHOD = local
MUSICOS_INIT_SUBDIR = crates/musicos-init
MUSICOS_INIT_OVERRIDE_SRCDIR_RSYNC_EXCLUSIONS = --exclude=out --exclude=vendor --exclude=target --exclude=.git
MUSICOS_INIT_DEPENDENCIES = host-rustc sqlite alsa-lib
MUSICOS_INIT_CARGO_BUILD_OPTS = --features pid1,device-audio

define MUSICOS_INIT_BUILD_CMDS
	cd $(@D)/$(MUSICOS_INIT_SUBDIR) && \
	$(TARGET_MAKE_ENV) \
	$(TARGET_CONFIGURE_OPTS) \
	$(PKG_CARGO_ENV) \
	$(MUSICOS_INIT_CARGO_ENV) \
	cargo build \
		--release \
		--manifest-path Cargo.toml \
		$(MUSICOS_INIT_CARGO_BUILD_OPTS)
endef

define MUSICOS_INIT_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/$(RUSTC_TARGET_NAME)/release/musicos-init $(TARGET_DIR)/sbin/init
	$(INSTALL) -d -m 0755 $(TARGET_DIR)/var/lib/musicos
endef

$(eval $(cargo-package))
