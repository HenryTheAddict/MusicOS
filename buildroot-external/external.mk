include $(sort $(wildcard $(BR2_EXTERNAL_MUSICOS_PATH)/package/*/*.mk))

define MUSICOS_RG35XX_AUDIO_DTS_FIXUP
	$(BR2_EXTERNAL_MUSICOS_PATH)/board/rg35xx/fixup-audio-dts.sh $(@D)
endef
LINUX_POST_PATCH_HOOKS += MUSICOS_RG35XX_AUDIO_DTS_FIXUP
