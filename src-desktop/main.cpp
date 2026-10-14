#include "uilib/Application.hpp"
#include <iostream>

int main(int argc, char* argv[]) {
    MusicUI::Application app;
    if (!app.init(argc, argv)) {
        std::cerr << "Failed to initialize MusicUI Application\n";
        return 1;
    }

    app.run();
    return 0;
}
