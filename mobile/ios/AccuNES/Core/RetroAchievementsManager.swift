import Foundation

final class RetroAchievementsManager: ObservableObject {

    @Published var username: String?
    @Published var isLoggedIn: Bool = false

    private let userDefaultsKey = "accunes_ra_username"
    private let tokenDefaultsKey = "accunes_ra_token"

    init() {
        self.username = UserDefaults.standard.string(forKey: userDefaultsKey)
        let token = UserDefaults.standard.string(forKey: tokenDefaultsKey)
        self.isLoggedIn = (username != nil && token != nil)
    }

    func login(user: String, pass: String, completion: @escaping (Bool) -> Void) {
        guard let url = URL(string: "https://retroachievements.org/dorequest.php?r=login&u=\(user)&p=\(pass)") else {
            completion(false)
            return
        }

        URLSession.shared.dataTask(with: url) { [weak self] data, _, error in
            guard let data = data, error == nil,
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let success = json["Success"] as? Bool, success else {
                DispatchQueue.main.async { completion(false) }
                return
            }

            let token = json["Token"] as? String ?? ""
            let loggedUser = json["User"] as? String ?? user

            UserDefaults.standard.set(loggedUser, forKey: self?.userDefaultsKey ?? "")
            UserDefaults.standard.set(token, forKey: self?.tokenDefaultsKey ?? "")

            DispatchQueue.main.async {
                self?.username = loggedUser
                self?.isLoggedIn = true
                completion(true)
            }
        }.resume()
    }

    func logout() {
        UserDefaults.standard.removeObject(forKey: userDefaultsKey)
        UserDefaults.standard.removeObject(forKey: tokenDefaultsKey)
        username = nil
        isLoggedIn = false
    }
}
