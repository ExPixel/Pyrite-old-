DEST := ../../roms/test
TARGETS := mode3 mode4 timer-stress
TARGETS_CLEAN := $(addsuffix .clean,$(TARGETS))

.PHONY: all clean $(TARGETS) $(TARGETS_CLEAN)

all: $(TARGETS)

clean: $(TARGETS_CLEAN)

$(TARGETS):
	cd $@; make
	cp $@/$@.gba $(DEST)

$(TARGETS_CLEAN):
	cd $(basename $@); make clean
