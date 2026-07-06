#include "utils/UrlUtils.h"

#include <gtest/gtest.h>

namespace fubuki {
namespace {

// ============================================================
// NormalizeNavigationInput — 基本動作
// ============================================================

TEST(NormalizeNavigationInputTest, EmptyInputReturnsNewTab) {
  EXPECT_EQ(NormalizeNavigationInput(""), "fubuki://newtab/");
}

TEST(NormalizeNavigationInputTest, WhitespaceOnlyReturnsNewTab) {
  EXPECT_EQ(NormalizeNavigationInput("   "), "fubuki://newtab/");
  EXPECT_EQ(NormalizeNavigationInput("\t"), "fubuki://newtab/");
  EXPECT_EQ(NormalizeNavigationInput("\n"), "fubuki://newtab/");
  EXPECT_EQ(NormalizeNavigationInput("\r\n"), "fubuki://newtab/");
  EXPECT_EQ(NormalizeNavigationInput("  \t\n  "), "fubuki://newtab/");
}

// ============================================================
// NormalizeNavigationInput — スキーマ付き URL の正規化
// ============================================================

TEST(NormalizeNavigationInputTest, PreservesHttpScheme) {
  EXPECT_EQ(NormalizeNavigationInput("http://example.com"), "http://example.com");
}

TEST(NormalizeNavigationInputTest, PreservesHttpsScheme) {
  EXPECT_EQ(NormalizeNavigationInput("https://example.com"), "https://example.com");
}

TEST(NormalizeNavigationInputTest, PreservesFubukiScheme) {
  EXPECT_EQ(NormalizeNavigationInput("fubuki://settings"), "fubuki://settings");
}

TEST(NormalizeNavigationInputTest, PreservesAboutScheme) {
  EXPECT_EQ(NormalizeNavigationInput("about:blank"), "about:blank");
}

TEST(NormalizeNavigationInputTest, PreservesFtpScheme) {
  EXPECT_EQ(NormalizeNavigationInput("ftp://files.example.com"), "ftp://files.example.com");
}

TEST(NormalizeNavigationInputTest, PreservesFileScheme) {
  EXPECT_EQ(NormalizeNavigationInput("file:///Users/test/index.html"), "file:///Users/test/index.html");
}

TEST(NormalizeNavigationInputTest, PreservesDataUriScheme) {
  EXPECT_EQ(NormalizeNavigationInput("data:text/html,<h1>Hello</h1>"),
            "data:text/html,<h1>Hello</h1>");
}

TEST(NormalizeNavigationInputTest, PreservesSchemeWithoutSlashes) {
  EXPECT_EQ(NormalizeNavigationInput("http:example.com"), "http:example.com");
  EXPECT_EQ(NormalizeNavigationInput("file:/Users/test/index.html"), "file:/Users/test/index.html");
}

TEST(NormalizeNavigationInputTest, PreservesUrlWithQueryAndFragment) {
  EXPECT_EQ(NormalizeNavigationInput("https://example.com/path?key=val#section"),
            "https://example.com/path?key=val#section");
}

TEST(NormalizeNavigationInputTest, PreservesUrlWithPort) {
  EXPECT_EQ(NormalizeNavigationInput("http://localhost:3000/"), "http://localhost:3000/");
}

TEST(NormalizeNavigationInputTest, SchemeInMiddleOfStringIsRecognized) {
  // HasScheme は "://" が文字列中のどこにあるかをチェックする
  EXPECT_EQ(NormalizeNavigationInput("foo://bar"), "foo://bar");
}

TEST(NormalizeNavigationInputTest, JustSchemePrefixIsRecognized) {
  EXPECT_EQ(NormalizeNavigationInput("http://"), "http://");
  EXPECT_EQ(NormalizeNavigationInput("https://"), "https://");
}

TEST(NormalizeNavigationInputTest, AboutPrefixWithoutBlankIsRecognized) {
  EXPECT_EQ(NormalizeNavigationInput("about:config"), "about:config");
}

TEST(NormalizeNavigationInputTest, FubukiPrefixWithoutPathIsRecognized) {
  EXPECT_EQ(NormalizeNavigationInput("fubuki:"), "fubuki:");
}

// ============================================================
// NormalizeNavigationInput — トリム
// ============================================================

TEST(NormalizeNavigationInputTest, TrimsLeadingWhitespace) {
  EXPECT_EQ(NormalizeNavigationInput("  https://example.com"), "https://example.com");
}

TEST(NormalizeNavigationInputTest, TrimsTrailingWhitespace) {
  EXPECT_EQ(NormalizeNavigationInput("https://example.com  "), "https://example.com");
}

TEST(NormalizeNavigationInputTest, TrimsLeadingAndTrailingWhitespace) {
  EXPECT_EQ(NormalizeNavigationInput("  https://example.com  "), "https://example.com");
}

TEST(NormalizeNavigationInputTest, TrimsTabsAndNewlines) {
  EXPECT_EQ(NormalizeNavigationInput("\t https://example.com \n"), "https://example.com");
}

// ============================================================
// NormalizeNavigationInput — ホスト名風入力
// ============================================================

TEST(NormalizeNavigationInputTest, PrependsHttpsToHostLikeInput) {
  EXPECT_EQ(NormalizeNavigationInput("example.com"), "https://example.com");
  EXPECT_EQ(NormalizeNavigationInput("sub.domain.org"), "https://sub.domain.org");
  EXPECT_EQ(NormalizeNavigationInput("my-site.co.jp"), "https://my-site.co.jp");
  EXPECT_EQ(NormalizeNavigationInput("example.com/path"), "https://example.com/path");
  EXPECT_EQ(NormalizeNavigationInput("example.com:8080/path?q=1"),
            "https://example.com:8080/path?q=1");
}

TEST(NormalizeNavigationInputTest, PrependsHttpToLocalIpAddress) {
  EXPECT_EQ(NormalizeNavigationInput("192.168.1.1"), "http://192.168.1.1");
  EXPECT_EQ(NormalizeNavigationInput("10.0.0.1"), "http://10.0.0.1");
  EXPECT_EQ(NormalizeNavigationInput("127.0.0.1"), "http://127.0.0.1");
  EXPECT_EQ(NormalizeNavigationInput("127.0.0.1:3000"), "http://127.0.0.1:3000");
}

TEST(NormalizeNavigationInputTest, PrependsHttpsToPublicIpAddress) {
  EXPECT_EQ(NormalizeNavigationInput("8.8.8.8"), "https://8.8.8.8");
}

TEST(NormalizeNavigationInputTest, InputWithDotButNoSpaceIsHostLike) {
  // "hello.world" → スペースなし + ドットあり → ホスト名扱い
  EXPECT_EQ(NormalizeNavigationInput("hello.world"), "https://hello.world");
}

TEST(NormalizeNavigationInputTest, PrependsHttpToLocalHosts) {
  EXPECT_EQ(NormalizeNavigationInput("localhost"), "http://localhost");
  EXPECT_EQ(NormalizeNavigationInput("localhost:5173"), "http://localhost:5173");
  EXPECT_EQ(NormalizeNavigationInput("[::1]:5173"), "http://[::1]:5173");
  EXPECT_EQ(NormalizeNavigationInput("nas.local"), "http://nas.local");
  EXPECT_EQ(NormalizeNavigationInput("router.local"), "http://router.local");
  EXPECT_EQ(NormalizeNavigationInput("proxmox.local"), "http://proxmox.local");
  EXPECT_EQ(NormalizeNavigationInput("devbox:8080"), "http://devbox:8080");
}

TEST(NormalizeNavigationInputTest, PrependsHttpsToIdnLikeHosts) {
  EXPECT_EQ(NormalizeNavigationInput("日本語.com"), "https://日本語.com");
  EXPECT_EQ(NormalizeNavigationInput("münich.example"), "https://münich.example");
}

// ============================================================
// NormalizeNavigationInput — 検索クエリ扱い
// ============================================================

TEST(NormalizeNavigationInputTest, InputWithSpacesIsSearchQuery) {
  EXPECT_EQ(NormalizeNavigationInput("hello world", "google"),
            "https://www.google.com/search?q=hello+world");
}

TEST(NormalizeNavigationInputTest, SingleWordIsSearchQuery) {
  // スペースなし・ドットなし → ホスト名でも URL でもない → 検索
  EXPECT_EQ(NormalizeNavigationInput("cats"), "https://www.google.com/search?q=cats");
}

TEST(NormalizeNavigationInputTest, InputWithoutDotIsSearchQuery) {
  EXPECT_EQ(NormalizeNavigationInput("192"), "https://www.google.com/search?q=192");
}

TEST(NormalizeNavigationInputTest, JapaneseTextIsSearchQuery) {
  EXPECT_EQ(NormalizeNavigationInput("日本語テスト", "google"),
            "https://www.google.com/search?q=%E6%97%A5%E6%9C%AC%E8%AA%9E%E3%83%86%E3%82%B9%E3%83%88");
}

TEST(NormalizeNavigationInputTest, JapaneseTextWithSpacesIsSearchQuery) {
  EXPECT_EQ(NormalizeNavigationInput("東京 天気", "google"),
            "https://www.google.com/search?q=%E6%9D%B1%E4%BA%AC+%E5%A4%A9%E6%B0%97");
  EXPECT_EQ(NormalizeNavigationInput("日本語 テスト", "google"),
            "https://www.google.com/search?q=%E6%97%A5%E6%9C%AC%E8%AA%9E+%E3%83%86%E3%82%B9%E3%83%88");
}

// ============================================================
// NormalizeNavigationInput — 検索エンジン切り替え
// ============================================================

TEST(NormalizeNavigationInputTest, SelectsGoogleByDefault) {
  EXPECT_EQ(NormalizeNavigationInput("cats"),
            "https://www.google.com/search?q=cats");
}

TEST(NormalizeNavigationInputTest, SelectsGoogleExplicitly) {
  EXPECT_EQ(NormalizeNavigationInput("cats", "google"),
            "https://www.google.com/search?q=cats");
}

TEST(NormalizeNavigationInputTest, SelectsBing) {
  EXPECT_EQ(NormalizeNavigationInput("cats", "bing"),
            "https://www.bing.com/search?q=cats");
}

TEST(NormalizeNavigationInputTest, FallsBackToDuckDuckGo) {
  EXPECT_EQ(NormalizeNavigationInput("cats", "unknown_engine"),
            "https://duckduckgo.com/?q=cats");
}

TEST(NormalizeNavigationInputTest, EmptyEngineFallsBackToDuckDuckGo) {
  EXPECT_EQ(NormalizeNavigationInput("cats", ""),
            "https://duckduckgo.com/?q=cats");
}

// ============================================================
// NormalizeNavigationInput — カスタム検索 URL
// ============================================================

TEST(NormalizeNavigationInputTest, CustomSearchUrlWithQueryPlaceholder) {
  EXPECT_EQ(
      NormalizeNavigationInput("rust lang", "custom",
                               "https://search.example.com/?query={query}"),
      "https://search.example.com/?query=rust+lang");
}

TEST(NormalizeNavigationInputTest, CustomSearchUrlWithPercentSPlaceholder) {
  EXPECT_EQ(
      NormalizeNavigationInput("rust lang", "custom",
                               "https://search.example.com/?q=%s"),
      "https://search.example.com/?q=rust+lang");
}

TEST(NormalizeNavigationInputTest, CustomSearchUrlWithoutPlaceholderAppendsQuery) {
  // ? がない場合 → `?q=` を追加
  EXPECT_EQ(
      NormalizeNavigationInput("test", "custom", "https://search.example.com"),
      "https://search.example.com?q=test");
}

TEST(NormalizeNavigationInputTest, CustomSearchUrlWithExistingQueryUsesAmpersand) {
  // ? が既にある場合 → `&q=` を追加
  EXPECT_EQ(
      NormalizeNavigationInput("test", "custom",
                               "https://search.example.com?foo=bar"),
      "https://search.example.com?foo=bar&q=test");
}

TEST(NormalizeNavigationInputTest, CustomSearchUrlEmptyUsesDuckDuckGo) {
  // customSearchUrl が空の場合 → フォールバック
  EXPECT_EQ(NormalizeNavigationInput("test", "custom", ""),
            "https://duckduckgo.com/?q=test");
}

TEST(NormalizeNavigationInputTest, CustomSearchUrlWithJapaneseQuery) {
  EXPECT_EQ(
      NormalizeNavigationInput("テスト", "custom",
                               "https://search.example.com/?q={query}"),
      "https://search.example.com/?q=%E3%83%86%E3%82%B9%E3%83%88");
}

TEST(NormalizeNavigationInputTest, CustomSearchUrlWithSpecialChars) {
  EXPECT_EQ(
      NormalizeNavigationInput("a&b=c", "custom",
                               "https://search.example.com/?q={query}"),
      "https://search.example.com/?q=a%26b%3Dc");
}

// ============================================================
// EscapeQuery — 基本動作
// ============================================================

TEST(EscapeQueryTest, EmptyStringReturnsEmpty) {
  EXPECT_EQ(EscapeQuery(""), "");
}

TEST(EscapeQueryTest, PassesThroughAlphanumeric) {
  EXPECT_EQ(EscapeQuery("abcXYZ0129"), "abcXYZ0129");
}

TEST(EscapeQueryTest, PassesThroughSafeCharacters) {
  // RFC 3986 の unreserved characters
  EXPECT_EQ(EscapeQuery("-_.~"), "-_.~");
}

TEST(EscapeQueryTest, EncodesSpacesAsPlus) {
  EXPECT_EQ(EscapeQuery("hello world"), "hello+world");
  EXPECT_EQ(EscapeQuery("  multiple  spaces  "), "++multiple++spaces++");
}

// ============================================================
// EscapeQuery — 特殊文字のエンコード
// ============================================================

TEST(EscapeQueryTest, EncodesAmpersand) {
  EXPECT_EQ(EscapeQuery("a&b"), "a%26b");
}

TEST(EscapeQueryTest, EncodesEquals) {
  EXPECT_EQ(EscapeQuery("a=b"), "a%3Db");
}

TEST(EscapeQueryTest, EncodesQuestionMark) {
  EXPECT_EQ(EscapeQuery("a?b"), "a%3Fb");
}

TEST(EscapeQueryTest, EncodesHash) {
  EXPECT_EQ(EscapeQuery("a#b"), "a%23b");
}

TEST(EscapeQueryTest, EncodesPercent) {
  // % は %25 にエンコードされる。二重エンコードされないこと。
  EXPECT_EQ(EscapeQuery("100%"), "100%25");
}

TEST(EscapeQueryTest, EncodesPlus) {
  // + はスペースの逆変換だが、EscapeQuery では + もエンコードされる
  EXPECT_EQ(EscapeQuery("a+b"), "a%2Bb");
}

TEST(EscapeQueryTest, EncodesSlash) {
  EXPECT_EQ(EscapeQuery("a/b"), "a%2Fb");
}

TEST(EscapeQueryTest, EncodesBackslash) {
  EXPECT_EQ(EscapeQuery("a\\b"), "a%5Cb");
}

TEST(EscapeQueryTest, EncodesColon) {
  EXPECT_EQ(EscapeQuery("a:b"), "a%3Ab");
}

TEST(EscapeQueryTest, EncodesSemicolon) {
  EXPECT_EQ(EscapeQuery("a;b"), "a%3Bb");
}

TEST(EscapeQueryTest, EncodesAt) {
  EXPECT_EQ(EscapeQuery("a@b"), "a%40b");
}

TEST(EscapeQueryTest, EncodesPipe) {
  EXPECT_EQ(EscapeQuery("a|b"), "a%7Cb");
}

TEST(EscapeQueryTest, EncodesAngleBrackets) {
  EXPECT_EQ(EscapeQuery("a<b"), "a%3Cb");
  EXPECT_EQ(EscapeQuery("a>b"), "a%3Eb");
}

TEST(EscapeQueryTest, EncodesCurlyBraces) {
  EXPECT_EQ(EscapeQuery("a{b"), "a%7Bb");
  EXPECT_EQ(EscapeQuery("a}b"), "a%7Db");
}

TEST(EscapeQueryTest, EncodesSquareBrackets) {
  EXPECT_EQ(EscapeQuery("a[b"), "a%5Bb");
  EXPECT_EQ(EscapeQuery("a]b"), "a%5Db");
}

TEST(EscapeQueryTest, EncodesCaret) {
  EXPECT_EQ(EscapeQuery("a^b"), "a%5Eb");
}

// ============================================================
// EscapeQuery — Unicode / 多バイト
// ============================================================

TEST(EscapeQueryTest, EncodesJapanese) {
  EXPECT_EQ(EscapeQuery("日本"), "%E6%97%A5%E6%9C%AC");
}

TEST(EscapeQueryTest, EncodesEmoji) {
  // 🐱 = U+1F431 = F0 9F 90 B1
  EXPECT_EQ(EscapeQuery("🐱"), "%F0%9F%90%B1");
}

TEST(EscapeQueryTest, EncodesFullWidthCharacters) {
  // Ａ = U+FF21 = EF BC A1
  EXPECT_EQ(EscapeQuery("Ａ"), "%EF%BC%A1");
}

TEST(EscapeQueryTest, EncodesNullByte) {
  EXPECT_EQ(EscapeQuery(std::string(1, '\0')), "%00");
}

TEST(EscapeQueryTest, EncodesTab) {
  EXPECT_EQ(EscapeQuery("\t"), "%09");
}

TEST(EscapeQueryTest, EncodesNewline) {
  EXPECT_EQ(EscapeQuery("\n"), "%0A");
}

TEST(EscapeQueryTest, EncodesCarriageReturn) {
  EXPECT_EQ(EscapeQuery("\r"), "%0D");
}

// ============================================================
// EscapeQuery — 混合入力
// ============================================================

TEST(EscapeQueryTest, MixedAsciiAndSpecialChars) {
  EXPECT_EQ(EscapeQuery("hello world & foo=bar?yes"),
            "hello+world+%26+foo%3Dbar%3Fyes");
}

TEST(EscapeQueryTest, LongStringIsFullyEncoded) {
  const std::string input(1000, 'a');
  const std::string expected(1000, 'a');
  EXPECT_EQ(EscapeQuery(input), expected);
}

TEST(EscapeQueryTest, AllAsciiPrintableCharacters) {
  // スペース(0x20)から ~(0x7E) まで、safe な文字はそのまま、それ以外は %XX
  for (int c = 0x20; c <= 0x7E; ++c) {
    const char ch = static_cast<char>(c);
    const std::string input(1, ch);
    const std::string result = EscapeQuery(input);

    if (ch == ' ') {
      EXPECT_EQ(result, "+") << "Space character (0x20)";
    } else if (std::isalnum(static_cast<unsigned char>(ch)) ||
               ch == '-' || ch == '_' || ch == '.' || ch == '~') {
      EXPECT_EQ(result, input) << "Safe character: " << ch;
    } else {
      // %XX 形式であることを確認
      EXPECT_EQ(result.size(), 3u) << "Encoded character: " << ch;
      EXPECT_EQ(result[0], '%') << "Encoded character: " << ch;
    }
  }
}

}  // namespace
}  // namespace fubuki
