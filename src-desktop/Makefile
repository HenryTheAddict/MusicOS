CC ?= clang
CXX ?= clang++
BREW_CFLAGS = -I/opt/homebrew/include -I/opt/homebrew/include/SDL3 -I/opt/homebrew/include/cairo -I/opt/homebrew/include/freetype2 -I/opt/homebrew/include/librsvg-2.0 -I/opt/homebrew/include/gdk-pixbuf-2.0 -I/opt/homebrew/include/glib-2.0 -I/opt/homebrew/lib/glib-2.0/include -I/opt/homebrew/include/pango-1.0 -I/opt/homebrew/include/harfbuzz
BREW_LIBS = -L/opt/homebrew/lib -lSDL3 -lSDL3_image -lcairo -lfreetype -lrsvg-2 -lgdk_pixbuf-2.0 -lglib-2.0 -lgobject-2.0 -lcurl -ljson-c

CFLAGS ?= -Wall -Wextra -O2 -g $(BREW_CFLAGS)
CXXFLAGS ?= -std=c++17 -MMD -MP -Wall -Wextra -O2 -g $(BREW_CFLAGS) -I.
LDFLAGS ?= $(BREW_LIBS) -framework ApplicationServices -framework CoreGraphics -framework CoreFoundation -framework CoreText -lm

CXX_SRCS = main.cpp \
           uilib/Canvas.cpp \
           uilib/components/HeaderComponents.cpp \
           uilib/components/NowPlayingPill.cpp \
           uilib/components/AlbumGrid.cpp \
           uilib/components/Sidebar.cpp \
           uilib/components/DeviceComponents.cpp \
           uilib/components/SparklistModal.cpp \
           uilib/Application.cpp \
           uilib/LibraryModel.cpp \
           uilib/MusicSearchService.cpp \
           uilib/LibraryExperience.cpp

OBJS = $(CXX_SRCS:.cpp=.o)
TARGET = musicos-desktop

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CXX) $(OBJS) $(LDFLAGS) -o $@

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(OBJS:.o=.d) $(TARGET)

-include $(OBJS:.o=.d)

.PHONY: all clean

../out/library-review/desktop-checks: $(filter-out main.o,$(OBJS)) tests/library_checks.cpp
	mkdir -p ../out/library-review
	$(CXX) $(CXXFLAGS) tests/library_checks.cpp $(filter-out main.o,$(OBJS)) $(LDFLAGS) -o ../out/library-review/desktop-checks

test: ../out/library-review/desktop-checks
	cd .. && ./out/library-review/desktop-checks

.PHONY: test
