#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <string>

#include "utils/DownloadUtils.h"

namespace fubuki {
namespace {

class DownloadUtilsTest : public ::testing::Test {
 protected:
  void SetUp() override {
    directory_ = std::filesystem::temp_directory_path() / "fubuki-download-utils-tests" /
                 ::testing::UnitTest::GetInstance()->current_test_info()->name();
    std::error_code error;
    std::filesystem::remove_all(directory_, error);
    ASSERT_TRUE(std::filesystem::create_directories(directory_));
  }

  void TearDown() override {
    std::error_code error;
    std::filesystem::remove_all(directory_, error);
  }

  std::filesystem::path directory_;
};

TEST_F(DownloadUtilsTest, UsesOnlyBasenameForTraversalAndAbsolutePaths) {
  EXPECT_EQ(SanitizeDownloadFilename("../../secret.txt"), "secret.txt");
  EXPECT_EQ(SanitizeDownloadFilename("/var/tmp/archive.zip"), "archive.zip");
  EXPECT_EQ(SanitizeDownloadFilename("../.."), "download");
  EXPECT_EQ(SanitizeDownloadFilename("../../"), "download");

  const auto path = UniqueDownloadPath(directory_, "../../outside.txt");
  EXPECT_EQ(path, directory_ / "outside.txt");
  EXPECT_EQ(path.parent_path(), directory_);
}

TEST_F(DownloadUtilsTest, RecognizesWindowsSeparatorsOnMacOS) {
  EXPECT_EQ(SanitizeDownloadFilename(R"(C:\Users\attacker\payload.exe)"), "payload.exe");
  EXPECT_EQ(SanitizeDownloadFilename(R"(..\..\notes.txt)"), "notes.txt");
  EXPECT_EQ(SanitizeDownloadFilename(R"(folder\)"), "download");
}

TEST_F(DownloadUtilsTest, PreservesNormalAndUnicodeNames) {
  EXPECT_EQ(SanitizeDownloadFilename("report 2026.pdf"), "report 2026.pdf");
  EXPECT_EQ(SanitizeDownloadFilename("レポート.pdf"), "レポート.pdf");
  EXPECT_EQ(SanitizeDownloadFilename("雪❄️.png"), "雪❄️.png");
}

TEST_F(DownloadUtilsTest, ReplacesControlAndUnsafeCharacters) {
  const std::string withNul("safe\0evil.txt", sizeof("safe\0evil.txt") - 1);
  EXPECT_EQ(SanitizeDownloadFilename(withNul), "safe_evil.txt");
  EXPECT_EQ(SanitizeDownloadFilename("line\nbreak.txt"), "line_break.txt");
  EXPECT_EQ(SanitizeDownloadFilename(R"(bad:<name>|?*.txt)"), "bad__name____.txt");
  EXPECT_EQ(SanitizeDownloadFilename(""), "download");
  EXPECT_EQ(SanitizeDownloadFilename("."), "download");
  EXPECT_EQ(SanitizeDownloadFilename(".."), "download");
  EXPECT_EQ(SanitizeDownloadFilename(" \t "), "download");
}

TEST_F(DownloadUtilsTest, AddsSuffixInsteadOfOverwritingExistingFiles) {
  std::ofstream(directory_ / "report.pdf").put('a');
  std::ofstream(directory_ / "report (1).pdf").put('b');

  const auto path = UniqueDownloadPath(directory_, "report.pdf");
  ASSERT_EQ(path, directory_ / "report (2).pdf");
  std::ofstream(path).put('c');

  char original = '\0';
  std::ifstream(directory_ / "report.pdf").get(original);
  EXPECT_EQ(original, 'a');
}

TEST_F(DownloadUtilsTest, TreatsExistingDirectoriesAsCollisions) {
  ASSERT_TRUE(std::filesystem::create_directory(directory_ / "download"));
  EXPECT_EQ(UniqueDownloadPath(directory_, ""), directory_ / "download (1)");
}

TEST_F(DownloadUtilsTest, DoesNotFollowDanglingSymlinkCollisions) {
  ASSERT_NO_THROW(
      std::filesystem::create_symlink(directory_ / "missing-target", directory_ / "report.pdf"));
  EXPECT_EQ(UniqueDownloadPath(directory_, "report.pdf"), directory_ / "report (1).pdf");
}

}  // namespace
}  // namespace fubuki
