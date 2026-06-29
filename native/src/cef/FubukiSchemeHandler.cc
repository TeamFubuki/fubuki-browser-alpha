#include "cef/FubukiSchemeHandler.h"

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <cstring>

#include "include/cef_parser.h"

namespace fubuki {

namespace {

std::string MimeForPath(const std::string& path) {
  if (path.ends_with(".html")) return "text/html";
  if (path.ends_with(".js")) return "application/javascript";
  if (path.ends_with(".css")) return "text/css";
  if (path.ends_with(".svg")) return "image/svg+xml";
  if (path.ends_with(".json")) return "application/json";
  if (path.ends_with(".png")) return "image/png";
  if (path.ends_with(".ico")) return "image/x-icon";
  return "application/octet-stream";
}

std::filesystem::path ProfilePath() {
  const char* home = std::getenv("HOME");
  return home ? std::filesystem::path(home) / "Library/Application Support/Fubuki Browser Alpha"
              : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
}

std::string ReadFile(const std::filesystem::path& path) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return "";
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  return buffer.str();
}

std::string HtmlEscape(const std::string& value) {
  std::ostringstream out;
  for (const char c : value) {
    switch (c) {
      case '&':
        out << "&amp;";
        break;
      case '<':
        out << "&lt;";
        break;
      case '>':
        out << "&gt;";
        break;
      case '"':
        out << "&quot;";
        break;
      case '\'':
        out << "&#39;";
        break;
      default:
        out << c;
        break;
    }
  }
  return out.str();
}

std::string DictString(CefRefPtr<CefDictionaryValue> dict, const std::string& key, const std::string& fallback = "") {
  return dict && dict->HasKey(key) && dict->GetType(key) == VTYPE_STRING ? dict->GetString(key).ToString() : fallback;
}

std::string FubukiLogoSvg(const std::string& className = "logo") {
  return "<svg class=\"" + className +
         "\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\">"
         "<path d=\"M128 440L183.252 248.366M470 72L252.28 72C238.617 72 226.68 81.2317 223.244 94.4554L183.252 248.366M183.252 248.366H363.904\" stroke=\"url(#paint0_linear_7_2)\" stroke-width=\"25\" stroke-linecap=\"round\"/>"
         "<path d=\"M95.6021 142.602L148.204 195.204M148.204 195.204L43.0001 195.204M148.204 195.204L95.6021 247.806M148.204 195.204V300.408M148.204 195.204L200.806 247.806M148.204 195.204V90M148.204 195.204L200.806 142.602M148.204 195.204H253.408\" stroke=\"#1AADEB\" stroke-width=\"5\" stroke-linecap=\"round\"/>"
         "<defs><linearGradient id=\"paint0_linear_7_2\" x1=\"257.282\" y1=\"72\" x2=\"257.282\" y2=\"476.326\" gradientUnits=\"userSpaceOnUse\"><stop stop-color=\"#FF9686\"/><stop offset=\"1\" stop-color=\"#A7ABE0\"/></linearGradient></defs>"
         "</svg>";
}

std::string NewTabHtml() {
  return std::string(R"(<!doctype html>
<html><head><meta charset="utf-8"><title>New Tab</title>
<style>
body{margin:0;font:15px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;background:#f8fafd;color:#1b1b1f;display:grid;place-items:center;height:100vh}
main{width:min(640px,calc(100vw - 48px));text-align:center}
.logo{width:86px;height:86px;margin:0 auto 18px;filter:drop-shadow(0 14px 28px rgb(24 52 90/.16))}
h1{font-size:30px;font-weight:720;margin:0 0 10px;letter-spacing:0}
p{color:#5d6470;margin:0;line-height:1.5}
</style></head><body><main>)") +
         FubukiLogoSvg() +
         R"(<h1>Fubuki Browser Alpha</h1><p>Enter a URL or search query in the address bar.</p></main></body></html>)";
}

std::string SettingsHtml() {
  const auto profile = ProfilePath();
  auto settingsValue = CefParseJSON(ReadFile(profile / "settings.json"), JSON_PARSER_RFC);
  CefRefPtr<CefDictionaryValue> settings = settingsValue && settingsValue->GetType() == VTYPE_DICTIONARY
                                               ? settingsValue->GetDictionary()
                                               : CefDictionaryValue::Create();
  auto bookmarkValue = CefParseJSON(ReadFile(profile / "bookmarks.json"), JSON_PARSER_RFC);
  CefRefPtr<CefListValue> bookmarks = bookmarkValue && bookmarkValue->GetType() == VTYPE_LIST
                                          ? bookmarkValue->GetList()
                                          : CefListValue::Create();

  const std::string homepage = DictString(settings, "homepage", "https://example.com");
  const std::string searchEngine = DictString(settings, "searchEngine", "duckduckgo");
  const std::string startupBehavior = DictString(settings, "startupBehavior", "homepage");
  const std::string downloadDirectory = DictString(settings, "downloadDirectory", "");
  const std::string theme = DictString(settings, "theme", "light");

  std::ostringstream bookmarksHtml;
  if (bookmarks->GetSize() == 0) {
    bookmarksHtml << "<p class=\"muted\">No bookmarks yet.</p>";
  }
  for (size_t i = 0; i < bookmarks->GetSize(); ++i) {
    auto item = bookmarks->GetDictionary(i);
    if (!item) continue;
    const std::string title = DictString(item, "title", DictString(item, "url", "Untitled"));
    const std::string url = DictString(item, "url");
    bookmarksHtml << "<div class=\"bookmark\"><a href=\"" << HtmlEscape(url) << "\"><strong>" << HtmlEscape(title)
                  << "</strong><span>" << HtmlEscape(url) << "</span></a><a class=\"text-button\" href=\"fubuki://settings/set?key=removeBookmark&value="
                  << CefURIEncode(url, false).ToString() << "\">Remove</a></div>";
  }

  std::ostringstream html;
  html << R"(<!doctype html><html><head><meta charset="utf-8"><title>Settings</title><style>
:root{color-scheme:light;--primary:#0b57d0;--on-primary:#fff;--surface:#f8fafd;--surface-container:#eef3fb;--surface-high:#e5ecf7;--outline:#c2cad6;--text:#191c20;--muted:#5d6470}
*{box-sizing:border-box}body{margin:0;background:var(--surface);color:var(--text);font:14px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;letter-spacing:0}
.settings{min-height:100vh;display:grid;grid-template-columns:260px minmax(0,1fr)}
aside{position:sticky;top:0;height:100vh;padding:28px 14px;background:var(--surface-container);border-right:1px solid var(--outline)}
.brand{display:flex;align-items:center;gap:12px;padding:0 12px 20px}.logo{width:38px;height:38px;flex:0 0 auto;filter:drop-shadow(0 7px 14px rgb(24 52 90/.14))}h1{font-size:22px;line-height:1.1;margin:0;font-weight:760}
nav{display:grid;gap:6px}nav a{height:44px;display:flex;align-items:center;padding:0 14px;border-radius:22px;color:var(--text);text-decoration:none;font-weight:650}nav a:hover,nav a.active{background:#d9e6ff;color:#073b8e}
main{padding:30px clamp(24px,4vw,54px) 56px;display:grid;gap:18px;align-content:start}.section{scroll-margin-top:20px}.card{background:#fff;border:1px solid var(--outline);border-radius:28px;padding:22px;box-shadow:0 1px 2px rgb(23 29 38/.05)}
h2{font-size:24px;margin:0 0 14px;font-weight:760}h3{font-size:16px;margin:0 0 10px;font-weight:720}.muted{color:var(--muted);line-height:1.5}.grid{display:grid;gap:14px}.row{display:grid;grid-template-columns:minmax(160px,.36fr) minmax(0,1fr);gap:16px;align-items:center;margin-top:12px}
input[type=text],select{width:100%;height:44px;border:1px solid var(--outline);border-radius:14px;padding:0 14px;background:#fff;color:var(--text);font:inherit}
.button{height:44px;border:0;border-radius:22px;padding:0 20px;background:var(--primary);color:var(--on-primary);font-weight:720;text-decoration:none;display:inline-grid;place-items:center}.text-button{color:var(--primary);font-weight:700;text-decoration:none}
.segmented{display:flex;flex-wrap:wrap;gap:8px}.chip{height:40px;border:1px solid var(--outline);border-radius:20px;padding:0 16px;display:inline-flex;align-items:center;color:var(--text);text-decoration:none;font-weight:650}.chip.selected{background:#d9e6ff;border-color:#9fc2ff;color:#073b8e}
.bookmark{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:14px;align-items:center;padding:12px 0;border-top:1px solid #e3e8ef}.bookmark:first-child{border-top:0}.bookmark a:first-child{min-width:0;color:var(--text);text-decoration:none;display:grid;gap:3px}.bookmark span{color:var(--muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.about{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.fact{background:var(--surface-container);border-radius:18px;padding:14px}.fact span{display:block;color:var(--muted);font-size:12px;margin-bottom:4px}.fact strong{font-weight:720;word-break:break-word}
@media(max-width:760px){.settings{grid-template-columns:1fr}aside{position:static;height:auto}.row,.about{grid-template-columns:1fr}}
</style></head><body><div class="settings"><aside><div class="brand">)"
       << FubukiLogoSvg() << R"(<h1>Settings</h1></div><nav>
<a class="active" href="#general">General</a><a href="#appearance">Appearance</a><a href="#search">Search</a><a href="#bookmarks">Bookmarks</a><a href="#privacy">Privacy</a><a href="#about">About</a>
</nav></aside><main>
<section id="general" class="section card"><h2>General</h2><p class="muted">Choose what opens when the browser starts and where downloads are saved.</p>
<form action="fubuki://settings/set" method="get" class="grid"><input type="hidden" name="key" value="homepage"><div class="row"><label>Homepage</label><input type="text" name="value" value=")"
       << HtmlEscape(homepage) << R"("></div><button class="button" type="submit">Save homepage</button></form>
<div class="row"><label>Startup</label><div class="segmented"><a class="chip)"
       << (startupBehavior == "homepage" ? " selected" : "") << R"(" href="fubuki://settings/set?key=startupBehavior&value=homepage">Homepage</a><a class="chip)"
       << (startupBehavior == "newTab" ? " selected" : "") << R"(" href="fubuki://settings/set?key=startupBehavior&value=newTab">New tab</a></div></div>
<div class="row"><label>Download folder</label><strong>)"
       << HtmlEscape(downloadDirectory) << R"(</strong></div></section>
<section id="appearance" class="section card"><h2>Appearance</h2><div class="row"><label>Theme</label><div class="segmented">
<a class="chip)"
       << (theme == "light" ? " selected" : "") << R"(" href="fubuki://settings/set?key=theme&value=light">Light</a><a class="chip)"
       << (theme == "soft" ? " selected" : "") << R"(" href="fubuki://settings/set?key=theme&value=soft">Soft</a><a class="chip)"
       << (theme == "muted" ? " selected" : "") << R"(" href="fubuki://settings/set?key=theme&value=muted">Muted</a><a class="chip)"
       << (theme == "dark" ? " selected" : "") << R"(" href="fubuki://settings/set?key=theme&value=dark">Dark</a></div></div></section>
<section id="search" class="section card"><h2>Search</h2><div class="row"><label>Default search engine</label><div class="segmented">
<a class="chip)"
       << (searchEngine == "duckduckgo" ? " selected" : "") << R"(" href="fubuki://settings/set?key=searchEngine&value=duckduckgo">DuckDuckGo</a><a class="chip)"
       << (searchEngine == "google" ? " selected" : "") << R"(" href="fubuki://settings/set?key=searchEngine&value=google">Google</a><a class="chip)"
       << (searchEngine == "bing" ? " selected" : "") << R"(" href="fubuki://settings/set?key=searchEngine&value=bing">Bing</a></div></div></section>
<section id="bookmarks" class="section card"><h2>Bookmarks</h2>)"
       << bookmarksHtml.str() << R"(</section>
<section id="privacy" class="section card"><h2>Privacy and security</h2><p class="muted">Permission prompts are denied by default in this alpha build. Downloads, history, bookmarks, and settings are stored locally in the browser profile.</p></section>
<section id="about" class="section card"><h2>About Fubuki</h2><div class="about"><div class="fact"><span>Browser version</span><strong>0.1.0 alpha</strong></div><div class="fact"><span>Engine</span><strong>Chromium Embedded Framework</strong></div><div class="fact"><span>Profile</span><strong>)"
       << HtmlEscape(profile.string()) << R"(</strong></div><div class="fact"><span>Bundle</span><strong>dev.fubuki.browser.alpha</strong></div></div></section>
</main></div></body></html>)";
  return html.str();
}

}  // namespace

FubukiSchemeHandler::FubukiSchemeHandler(std::string uiDistPath) : uiDistPath_(std::move(uiDistPath)) {}

bool FubukiSchemeHandler::Open(CefRefPtr<CefRequest> request, bool& handle_request, CefRefPtr<CefCallback>) {
  handle_request = true;
  return LoadRequest(request->GetURL().ToString());
}

void FubukiSchemeHandler::GetResponseHeaders(CefRefPtr<CefResponse> response,
                                             int64_t& response_length,
                                             CefString&) {
  response->SetStatus(status_);
  response->SetMimeType(mimeType_);
  CefResponse::HeaderMap headers;
  if (mimeType_.rfind("text/", 0) == 0 || mimeType_ == "application/javascript" || mimeType_ == "application/json") {
    headers.insert({"Content-Type", mimeType_ + "; charset=utf-8"});
  } else {
    headers.insert({"Content-Type", mimeType_});
  }
  headers.insert({"Cache-Control", "no-store, max-age=0"});
  response->SetHeaderMap(headers);
  response_length = static_cast<int64_t>(data_.size());
}

bool FubukiSchemeHandler::Read(void* data_out, int bytes_to_read, int& bytes_read, CefRefPtr<CefResourceReadCallback>) {
  const size_t remaining = data_.size() - offset_;
  const size_t count = std::min<size_t>(remaining, static_cast<size_t>(bytes_to_read));
  if (count > 0) {
    std::memcpy(data_out, data_.data() + offset_, count);
    offset_ += count;
    bytes_read = static_cast<int>(count);
    return true;
  }
  bytes_read = 0;
  return false;
}

void FubukiSchemeHandler::Cancel() {}

bool FubukiSchemeHandler::LoadRequest(const std::string& url) {
  offset_ = 0;
  if (url.rfind("fubuki://newtab/", 0) == 0) {
    LoadText(NewTabHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://settings", 0) == 0) {
    LoadText(SettingsHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://app/", 0) == 0) {
    const std::string path = ResolveAppPath(url);
    if (LoadFile(path, MimeForPath(path))) {
      return true;
    }
    LoadText("Fubuki UI build not found. Run `npm install && npm run build` in ui/.", "text/plain", 404);
    return true;
  }
  LoadText("Not found", "text/plain", 404);
  return true;
}

bool FubukiSchemeHandler::LoadFile(const std::string& path, const std::string& mimeType) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return false;
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  LoadText(buffer.str(), mimeType, 200);
  return true;
}

void FubukiSchemeHandler::LoadText(std::string body, std::string mimeType, int status) {
  data_ = std::move(body);
  mimeType_ = std::move(mimeType);
  status_ = status;
}

std::string FubukiSchemeHandler::ResolveAppPath(const std::string& url) const {
  std::string path = url.substr(std::string("fubuki://app/").size());
  const size_t query = path.find_first_of("?#");
  if (query != std::string::npos) {
    path = path.substr(0, query);
  }
  if (path.empty() || path == "/") {
    path = "index.html";
  }
  if (path.find("..") != std::string::npos) {
    path = "index.html";
  }
  return uiDistPath_ + "/" + path;
}

FubukiSchemeHandlerFactory::FubukiSchemeHandlerFactory(std::string uiDistPath)
    : uiDistPath_(std::move(uiDistPath)) {}

CefRefPtr<CefResourceHandler> FubukiSchemeHandlerFactory::Create(CefRefPtr<CefBrowser>,
                                                                 CefRefPtr<CefFrame>,
                                                                 const CefString&,
                                                                 CefRefPtr<CefRequest>) {
  return new FubukiSchemeHandler(uiDistPath_);
}

}  // namespace fubuki
