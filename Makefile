TARGETS := test
TARGETS_CLEAN := $(addsuffix .clean,$(TARGETS))

.PHONY: all clean $(TARGETS) $(TARGETS_CLEAN)

all: $(TARGETS)

clean: $(TARGETS_CLEAN)

$(TARGETS):
	cd roms-src/$@; make

$(TARGETS_CLEAN):
	cd roms-src/$(basename $@); make clean

