import SwiftUI
import UniformTypeIdentifiers

struct RecentGame: Identifiable, Hashable {
    let id = UUID()
    let title: String
    let url: URL
}

struct ROMPickerView: View {

    @Binding var recentGames: [RecentGame]
    let onSelectROM: (URL, String) -> Void

    @State private var showFilePicker = false

    var body: some View {
        NavigationView {
            List {
                Section(header: Text("Recent Games")) {
                    if recentGames.isEmpty {
                        VStack(spacing: 8) {
                            Image(systemName: "gamecontroller")
                                .font(.system(size: 44))
                                .foregroundColor(.secondary)
                            Text("No recent games")
                                .font(.headline)
                                .foregroundColor(.secondary)
                            Text("Tap 'Open ROM' to select a .nes or .fds file")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.vertical, 32)
                    } else {
                        ForEach(recentGames) { game in
                            Button(action: {
                                onSelectROM(game.url, game.title)
                            }) {
                                HStack(spacing: 12) {
                                    Image(systemName: "gamecontroller.fill")
                                        .foregroundColor(Color(red: 229/255, green: 37/255, blue: 33/255))
                                    Text(game.title)
                                        .font(.body)
                                        .foregroundColor(.primary)
                                }
                                .padding(.vertical, 4)
                            }
                        }
                    }
                }
            }
            .navigationTitle("AccuNES")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button(action: { showFilePicker = true }) {
                        Label("Open ROM", systemImage: "folder.badge.plus")
                    }
                }
            }
            .fileImporter(
                isPresented: $showFilePicker,
                allowedContentTypes: [.data, .item],
                allowsMultipleSelection: false
            ) { result in
                switch result {
                case .success(let urls):
                    guard let url = urls.first else { return }
                    let secured = url.startAccessingSecurityScopedResource()
                    defer { if secured { url.stopAccessingSecurityScopedResource() } }

                    let name = url.deletingPathExtension().lastPathComponent
                    onSelectROM(url, name)
                case .failure:
                    break
                }
            }
        }
    }
}
