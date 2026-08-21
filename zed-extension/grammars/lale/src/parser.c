#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 5
#define LARGE_STATE_COUNT 4
#define SYMBOL_COUNT 5
#define ALIAS_COUNT 0
#define TOKEN_COUNT 3
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 2
#define PRODUCTION_ID_COUNT 1

enum ts_symbol_identifiers {
  sym__ws = 1,
  sym_token = 2,
  sym_source_file = 3,
  aux_sym_source_file_repeat1 = 4,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [sym__ws] = "_ws",
  [sym_token] = "token",
  [sym_source_file] = "source_file",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [sym__ws] = sym__ws,
  [sym_token] = sym_token,
  [sym_source_file] = sym_source_file,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [sym__ws] = {
    .visible = false,
    .named = true,
  },
  [sym_token] = {
    .visible = true,
    .named = true,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(149);
      ADVANCE_MAP(
        '\n', 150,
        '!', 173,
        '"', 162,
        '#', 187,
        '%', 173,
        '\'', 289,
        '*', 173,
        '+', 173,
        '-', 282,
        '/', 165,
        '0', 163,
        '<', 174,
        '=', 173,
        '>', 173,
        'a', 225,
        'b', 213,
        'c', 185,
        'd', 194,
        'e', 228,
        'f', 169,
        'h', 180,
        'i', 168,
        'l', 243,
        'm', 183,
        'n', 242,
        'o', 278,
        'p', 236,
        'r', 195,
        's', 214,
        't', 201,
        'u', 167,
        'v', 177,
        'w', 178,
        'x', 239,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(150);
      if (('1' <= lookahead && lookahead <= '9')) ADVANCE(164);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('g' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      if (lookahead != 0) ADVANCE(151);
      END_STATE();
    case 1:
      if (lookahead == ' ') ADVANCE(25);
      END_STATE();
    case 2:
      if (lookahead == ' ') ADVANCE(135);
      END_STATE();
    case 3:
      if (lookahead == ' ') ADVANCE(90);
      END_STATE();
    case 4:
      if (lookahead == '"') ADVANCE(151);
      if (lookahead == '\\') ADVANCE(148);
      if (lookahead == '{') ADVANCE(142);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 5:
      if (lookahead == '\'') ADVANCE(151);
      END_STATE();
    case 6:
      if (lookahead == '.') ADVANCE(145);
      if (('0' <= lookahead && lookahead <= '9') ||
          lookahead == '_') ADVANCE(6);
      END_STATE();
    case 7:
      if (lookahead == '>') ADVANCE(151);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '\r') ADVANCE(7);
      END_STATE();
    case 8:
      if (lookahead == '_') ADVANCE(44);
      END_STATE();
    case 9:
      if (lookahead == '_') ADVANCE(136);
      END_STATE();
    case 10:
      if (lookahead == '_') ADVANCE(84);
      END_STATE();
    case 11:
      if (lookahead == '_') ADVANCE(126);
      if (lookahead == 'r') ADVANCE(9);
      END_STATE();
    case 12:
      if (lookahead == 'a') ADVANCE(73);
      END_STATE();
    case 13:
      if (lookahead == 'a') ADVANCE(71);
      END_STATE();
    case 14:
      if (lookahead == 'a') ADVANCE(75);
      END_STATE();
    case 15:
      if (lookahead == 'a') ADVANCE(121);
      END_STATE();
    case 16:
      if (lookahead == 'a') ADVANCE(115);
      END_STATE();
    case 17:
      if (lookahead == 'a') ADVANCE(118);
      if (lookahead == 'o') ADVANCE(41);
      END_STATE();
    case 18:
      if (lookahead == 'a') ADVANCE(113);
      END_STATE();
    case 19:
      if (lookahead == 'a') ADVANCE(125);
      END_STATE();
    case 20:
      if (lookahead == 'a') ADVANCE(80);
      if (lookahead == 'o') ADVANCE(104);
      if (lookahead == 'x') ADVANCE(94);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(20);
      END_STATE();
    case 21:
      if (lookahead == 'b') ADVANCE(129);
      END_STATE();
    case 22:
      if (lookahead == 'c') ADVANCE(124);
      END_STATE();
    case 23:
      if (lookahead == 'c') ADVANCE(49);
      END_STATE();
    case 24:
      if (lookahead == 'c') ADVANCE(34);
      END_STATE();
    case 25:
      if (lookahead == 'c') ADVANCE(16);
      if (lookahead == 's') ADVANCE(134);
      END_STATE();
    case 26:
      if (lookahead == 'c') ADVANCE(95);
      END_STATE();
    case 27:
      if (lookahead == 'c') ADVANCE(18);
      END_STATE();
    case 28:
      if (lookahead == 'd') ADVANCE(151);
      END_STATE();
    case 29:
      if (lookahead == 'd') ADVANCE(92);
      END_STATE();
    case 30:
      if (lookahead == 'd') ADVANCE(31);
      END_STATE();
    case 31:
      if (lookahead == 'e') ADVANCE(151);
      END_STATE();
    case 32:
      if (lookahead == 'e') ADVANCE(21);
      END_STATE();
    case 33:
      if (lookahead == 'e') ADVANCE(3);
      END_STATE();
    case 34:
      if (lookahead == 'e') ADVANCE(8);
      END_STATE();
    case 35:
      if (lookahead == 'e') ADVANCE(11);
      END_STATE();
    case 36:
      if (lookahead == 'e') ADVANCE(117);
      if (lookahead == 'y') ADVANCE(103);
      END_STATE();
    case 37:
      if (lookahead == 'e') ADVANCE(43);
      END_STATE();
    case 38:
      ADVANCE_MAP(
        'e', 82,
        'f', 76,
        'i', 41,
        'l', 99,
        'm', 15,
        's', 138,
        't', 36,
        'w', 50,
      );
      END_STATE();
    case 39:
      if (lookahead == 'e') ADVANCE(76);
      END_STATE();
    case 40:
      if (lookahead == 'e') ADVANCE(109);
      END_STATE();
    case 41:
      if (lookahead == 'f') ADVANCE(151);
      END_STATE();
    case 42:
      if (lookahead == 'f') ADVANCE(118);
      END_STATE();
    case 43:
      if (lookahead == 'f') ADVANCE(120);
      END_STATE();
    case 44:
      if (lookahead == 'f') ADVANCE(66);
      if (lookahead == 'l') ADVANCE(65);
      END_STATE();
    case 45:
      if (lookahead == 'g') ADVANCE(151);
      END_STATE();
    case 46:
      if (lookahead == 'g') ADVANCE(51);
      END_STATE();
    case 47:
      if (lookahead == 'g') ADVANCE(106);
      END_STATE();
    case 48:
      if (lookahead == 'g') ADVANCE(86);
      END_STATE();
    case 49:
      if (lookahead == 'h') ADVANCE(151);
      END_STATE();
    case 50:
      if (lookahead == 'h') ADVANCE(39);
      END_STATE();
    case 51:
      if (lookahead == 'h') ADVANCE(120);
      END_STATE();
    case 52:
      if (lookahead == 'h') ADVANCE(67);
      END_STATE();
    case 53:
      if (lookahead == 'i') ADVANCE(141);
      if (lookahead == 'o') ADVANCE(128);
      END_STATE();
    case 54:
      if (lookahead == 'i') ADVANCE(119);
      END_STATE();
    case 55:
      if (lookahead == 'i') ADVANCE(139);
      END_STATE();
    case 56:
      if (lookahead == 'i') ADVANCE(69);
      END_STATE();
    case 57:
      if (lookahead == 'i') ADVANCE(48);
      END_STATE();
    case 58:
      if (lookahead == 'i') ADVANCE(78);
      END_STATE();
    case 59:
      if (lookahead == 'i') ADVANCE(75);
      END_STATE();
    case 60:
      if (lookahead == 'i') ADVANCE(121);
      END_STATE();
    case 61:
      if (lookahead == 'i') ADVANCE(46);
      END_STATE();
    case 62:
      if (lookahead == 'i') ADVANCE(97);
      END_STATE();
    case 63:
      if (lookahead == 'i') ADVANCE(123);
      END_STATE();
    case 64:
      if (lookahead == 'i') ADVANCE(98);
      END_STATE();
    case 65:
      if (lookahead == 'i') ADVANCE(85);
      END_STATE();
    case 66:
      if (lookahead == 'i') ADVANCE(70);
      END_STATE();
    case 67:
      if (lookahead == 'i') ADVANCE(42);
      END_STATE();
    case 68:
      if (lookahead == 'l') ADVANCE(99);
      if (lookahead == 'p') ADVANCE(108);
      END_STATE();
    case 69:
      if (lookahead == 'l') ADVANCE(35);
      END_STATE();
    case 70:
      if (lookahead == 'l') ADVANCE(31);
      END_STATE();
    case 71:
      if (lookahead == 'l') ADVANCE(133);
      END_STATE();
    case 72:
      if (lookahead == 'l') ADVANCE(37);
      if (lookahead == 'r') ADVANCE(61);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(72);
      END_STATE();
    case 73:
      if (lookahead == 'm') ADVANCE(151);
      END_STATE();
    case 74:
      if (lookahead == 'm') ADVANCE(102);
      END_STATE();
    case 75:
      if (lookahead == 'm') ADVANCE(31);
      END_STATE();
    case 76:
      if (lookahead == 'n') ADVANCE(151);
      END_STATE();
    case 77:
      if (lookahead == 'n') ADVANCE(22);
      END_STATE();
    case 78:
      if (lookahead == 'n') ADVANCE(29);
      END_STATE();
    case 79:
      if (lookahead == 'n') ADVANCE(54);
      END_STATE();
    case 80:
      if (lookahead == 'n') ADVANCE(28);
      END_STATE();
    case 81:
      if (lookahead == 'n') ADVANCE(10);
      END_STATE();
    case 82:
      if (lookahead == 'n') ADVANCE(131);
      END_STATE();
    case 83:
      if (lookahead == 'n') ADVANCE(91);
      if (lookahead == 'v') ADVANCE(13);
      END_STATE();
    case 84:
      if (lookahead == 'n') ADVANCE(14);
      END_STATE();
    case 85:
      if (lookahead == 'n') ADVANCE(31);
      END_STATE();
    case 86:
      if (lookahead == 'n') ADVANCE(19);
      END_STATE();
    case 87:
      if (lookahead == 'o') ADVANCE(151);
      END_STATE();
    case 88:
      if (lookahead == 'o') ADVANCE(74);
      END_STATE();
    case 89:
      if (lookahead == 'o') ADVANCE(111);
      END_STATE();
    case 90:
      if (lookahead == 'o') ADVANCE(41);
      END_STATE();
    case 91:
      if (lookahead == 'o') ADVANCE(2);
      END_STATE();
    case 92:
      if (lookahead == 'o') ADVANCE(137);
      END_STATE();
    case 93:
      if (lookahead == 'o') ADVANCE(100);
      END_STATE();
    case 94:
      if (lookahead == 'o') ADVANCE(104);
      END_STATE();
    case 95:
      if (lookahead == 'o') ADVANCE(30);
      END_STATE();
    case 96:
      if (lookahead == 'o') ADVANCE(47);
      END_STATE();
    case 97:
      if (lookahead == 'o') ADVANCE(76);
      END_STATE();
    case 98:
      if (lookahead == 'o') ADVANCE(81);
      END_STATE();
    case 99:
      if (lookahead == 'o') ADVANCE(93);
      END_STATE();
    case 100:
      if (lookahead == 'p') ADVANCE(151);
      END_STATE();
    case 101:
      if (lookahead == 'p') ADVANCE(33);
      END_STATE();
    case 102:
      if (lookahead == 'p') ADVANCE(56);
      END_STATE();
    case 103:
      if (lookahead == 'p') ADVANCE(31);
      END_STATE();
    case 104:
      if (lookahead == 'r') ADVANCE(151);
      END_STATE();
    case 105:
      if (lookahead == 'r') ADVANCE(24);
      END_STATE();
    case 106:
      if (lookahead == 'r') ADVANCE(12);
      END_STATE();
    case 107:
      if (lookahead == 'r') ADVANCE(31);
      END_STATE();
    case 108:
      if (lookahead == 'r') ADVANCE(96);
      END_STATE();
    case 109:
      if (lookahead == 'r') ADVANCE(116);
      END_STATE();
    case 110:
      if (lookahead == 's') ADVANCE(151);
      END_STATE();
    case 111:
      if (lookahead == 's') ADVANCE(55);
      END_STATE();
    case 112:
      if (lookahead == 's') ADVANCE(57);
      END_STATE();
    case 113:
      if (lookahead == 's') ADVANCE(118);
      END_STATE();
    case 114:
      if (lookahead == 's') ADVANCE(52);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(114);
      END_STATE();
    case 115:
      if (lookahead == 's') ADVANCE(31);
      END_STATE();
    case 116:
      if (lookahead == 's') ADVANCE(62);
      END_STATE();
    case 117:
      if (lookahead == 's') ADVANCE(122);
      END_STATE();
    case 118:
      if (lookahead == 't') ADVANCE(151);
      END_STATE();
    case 119:
      if (lookahead == 't') ADVANCE(3);
      END_STATE();
    case 120:
      if (lookahead == 't') ADVANCE(144);
      END_STATE();
    case 121:
      if (lookahead == 't') ADVANCE(23);
      END_STATE();
    case 122:
      if (lookahead == 't') ADVANCE(1);
      END_STATE();
    case 123:
      if (lookahead == 't') ADVANCE(31);
      END_STATE();
    case 124:
      if (lookahead == 't') ADVANCE(64);
      END_STATE();
    case 125:
      if (lookahead == 't') ADVANCE(132);
      END_STATE();
    case 126:
      if (lookahead == 't') ADVANCE(59);
      END_STATE();
    case 127:
      if (lookahead == 't') ADVANCE(87);
      END_STATE();
    case 128:
      if (lookahead == 'u') ADVANCE(105);
      END_STATE();
    case 129:
      if (lookahead == 'u') ADVANCE(45);
      END_STATE();
    case 130:
      if (lookahead == 'u') ADVANCE(77);
      END_STATE();
    case 131:
      if (lookahead == 'u') ADVANCE(73);
      END_STATE();
    case 132:
      if (lookahead == 'u') ADVANCE(107);
      END_STATE();
    case 133:
      if (lookahead == 'u') ADVANCE(31);
      END_STATE();
    case 134:
      if (lookahead == 'u') ADVANCE(63);
      END_STATE();
    case 135:
      if (lookahead == 'v') ADVANCE(13);
      END_STATE();
    case 136:
      if (lookahead == 'v') ADVANCE(40);
      END_STATE();
    case 137:
      if (lookahead == 'w') ADVANCE(110);
      END_STATE();
    case 138:
      if (lookahead == 'w') ADVANCE(60);
      END_STATE();
    case 139:
      if (lookahead == 'x') ADVANCE(151);
      END_STATE();
    case 140:
      if (lookahead == 'y') ADVANCE(101);
      END_STATE();
    case 141:
      if (lookahead == 'z') ADVANCE(33);
      END_STATE();
    case 142:
      if (lookahead == '}') ADVANCE(4);
      if (lookahead != 0) ADVANCE(142);
      END_STATE();
    case 143:
      if (lookahead == '+' ||
          lookahead == '-') ADVANCE(146);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(286);
      END_STATE();
    case 144:
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(114);
      END_STATE();
    case 145:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(281);
      END_STATE();
    case 146:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(286);
      END_STATE();
    case 147:
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(287);
      END_STATE();
    case 148:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(4);
      END_STATE();
    case 149:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 150:
      ACCEPT_TOKEN(sym__ws);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(150);
      END_STATE();
    case 151:
      ACCEPT_TOKEN(sym_token);
      END_STATE();
    case 152:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(38);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 153:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(83);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 154:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(68);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 155:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(25);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 156:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(17);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 157:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(27);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 158:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(26);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 159:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(97);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 160:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(127);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 161:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == ' ') ADVANCE(112);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 162:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '"') ADVANCE(151);
      if (lookahead == '\\') ADVANCE(148);
      if (lookahead == '{') ADVANCE(142);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 163:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '.') ADVANCE(145);
      if (lookahead == '_') ADVANCE(6);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(143);
      if (lookahead == 'X' ||
          lookahead == 'x') ADVANCE(147);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(164);
      END_STATE();
    case 164:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '.') ADVANCE(145);
      if (lookahead == '_') ADVANCE(6);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(143);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(164);
      END_STATE();
    case 165:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '/') ADVANCE(166);
      if (lookahead == '=') ADVANCE(151);
      END_STATE();
    case 166:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '/') ADVANCE(290);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(290);
      END_STATE();
    case 167:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '1') ADVANCE(172);
      if (lookahead == '3') ADVANCE(170);
      if (lookahead == '6') ADVANCE(171);
      if (lookahead == '8') ADVANCE(288);
      if (lookahead == 'n') ADVANCE(254);
      if (lookahead == 's') ADVANCE(193);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 168:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '1') ADVANCE(172);
      if (lookahead == '3') ADVANCE(170);
      if (lookahead == '6') ADVANCE(171);
      if (lookahead == 'm') ADVANCE(248);
      if (lookahead == 'n') ADVANCE(275);
      if (lookahead == '8' ||
          lookahead == 'f') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 169:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '1') ADVANCE(172);
      if (lookahead == '3') ADVANCE(170);
      if (lookahead == '6') ADVANCE(171);
      if (lookahead == 'n') ADVANCE(161);
      if (lookahead == 'r') ADVANCE(238);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 170:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '2') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 171:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '4') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 172:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '6') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 173:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '=') ADVANCE(151);
      END_STATE();
    case 174:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '=') ADVANCE(175);
      if (lookahead == '>') ADVANCE(151);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '\r') ADVANCE(7);
      END_STATE();
    case 175:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '>') ADVANCE(151);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '\r') ADVANCE(7);
      END_STATE();
    case 176:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(190);
      if (lookahead == 't') ADVANCE(271);
      if (lookahead == 'w') ADVANCE(219);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 177:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(227);
      if (lookahead == 'e') ADVANCE(188);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 178:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(251);
      if (lookahead == 'h') ADVANCE(200);
      if (lookahead == 'r') ADVANCE(216);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 179:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(207);
      if (lookahead == 'i') ADVANCE(209);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 180:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(257);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 181:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(267);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 182:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(249);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 183:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(266);
      if (lookahead == 'i') ADVANCE(258);
      if (lookahead == 'o') ADVANCE(277);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 184:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(273);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 185:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'a') ADVANCE(256);
      if (lookahead == 'h') ADVANCE(182);
      if (lookahead == 'o') ADVANCE(246);
      if (lookahead == 'r') ADVANCE(244);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 186:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'b') ADVANCE(272);
      if (lookahead == 'f') ADVANCE(184);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 187:
      ACCEPT_TOKEN(sym_token);
      ADVANCE_MAP(
        'c', 88,
        'd', 32,
        'f', 130,
        'p', 89,
        's', 53,
        't', 140,
        'u', 79,
        'w', 58,
      );
      END_STATE();
    case 188:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'c') ADVANCE(283);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 189:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'c') ADVANCE(211);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 190:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'd') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 191:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'd') ADVANCE(284);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 192:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'd') ADVANCE(152);
      if (lookahead == 'u') ADVANCE(229);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 193:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 194:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(186);
      if (lookahead == 'o') ADVANCE(263);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 195:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(176);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 196:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(191);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 197:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(249);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 198:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(285);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 199:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(250);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 200:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(230);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 201:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(261);
      if (lookahead == 'o') ADVANCE(288);
      if (lookahead == 'y') ADVANCE(247);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 202:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(245);
      if (lookahead == 'r') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 203:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(252);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 204:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(159);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 205:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(156);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 206:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'e') ADVANCE(157);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 207:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'f') ADVANCE(206);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 208:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'g') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 209:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'g') ADVANCE(234);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 210:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'g') ADVANCE(158);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 211:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'h') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 212:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'h') ADVANCE(222);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 213:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(264);
      if (lookahead == 'o') ADVANCE(237);
      if (lookahead == 'y') ADVANCE(267);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 214:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(209);
      if (lookahead == 't') ADVANCE(202);
      if (lookahead == 'w') ADVANCE(218);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 215:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(235);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 216:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(267);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 217:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(276);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 218:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(266);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 219:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(231);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 220:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(233);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 221:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(268);
      if (lookahead == 'p') ADVANCE(240);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 222:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(232);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 223:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'i') ADVANCE(259);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 224:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'l') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 225:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'l') ADVANCE(199);
      if (lookahead == 'n') ADVANCE(190);
      if (lookahead == 's') ADVANCE(255);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 226:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'l') ADVANCE(263);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 227:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'l') ADVANCE(274);
      if (lookahead == 'r') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 228:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'l') ADVANCE(256);
      if (lookahead == 'n') ADVANCE(192);
      if (lookahead == 'x') ADVANCE(221);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 229:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'm') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 230:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'n') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 231:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'n') ADVANCE(190);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 232:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'n') ADVANCE(208);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 233:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'n') ADVANCE(210);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 234:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'n') ADVANCE(196);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 235:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'n') ADVANCE(270);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 236:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(215);
      if (lookahead == 'r') ADVANCE(217);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 237:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(224);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 238:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(229);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 239:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(249);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 240:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(250);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 241:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(245);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 242:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(265);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 243:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(241);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 244:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'o') ADVANCE(260);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 245:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'p') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 246:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'p') ADVANCE(280);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 247:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'p') ADVANCE(193);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 248:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'p') ADVANCE(240);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 249:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'r') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 250:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'r') ADVANCE(263);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 251:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'r') ADVANCE(230);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 252:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'r') ADVANCE(160);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 253:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 254:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(179);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 255:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(199);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 256:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(193);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 257:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(153);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 258:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(262);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 259:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(198);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 260:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(253);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 261:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(269);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 262:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 's') ADVANCE(220);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 263:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 264:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(279);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 265:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(212);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 266:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(189);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 267:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(193);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 268:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(154);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 269:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(155);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 270:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 't') ADVANCE(203);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 271:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'u') ADVANCE(251);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 272:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'u') ADVANCE(208);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 273:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'u') ADVANCE(226);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 274:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'u') ADVANCE(205);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 275:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'v') ADVANCE(199);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 276:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'v') ADVANCE(181);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 277:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'v') ADVANCE(204);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 278:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'v') ADVANCE(197);
      if (lookahead == 'f' ||
          lookahead == 'r') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 279:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'w') ADVANCE(223);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 280:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'y') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 281:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(143);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(281);
      END_STATE();
    case 282:
      ACCEPT_TOKEN(sym_token);
      if (lookahead == '=' ||
          lookahead == '>') ADVANCE(151);
      END_STATE();
    case 283:
      ACCEPT_TOKEN(sym_token);
      if (('2' <= lookahead && lookahead <= '4')) ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 284:
      ACCEPT_TOKEN(sym_token);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(72);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 285:
      ACCEPT_TOKEN(sym_token);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(20);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 286:
      ACCEPT_TOKEN(sym_token);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(286);
      END_STATE();
    case 287:
      ACCEPT_TOKEN(sym_token);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'F') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'f')) ADVANCE(287);
      END_STATE();
    case 288:
      ACCEPT_TOKEN(sym_token);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          (0xc0 <= lookahead && lookahead <= 0xffff)) ADVANCE(288);
      END_STATE();
    case 289:
      ACCEPT_TOKEN(sym_token);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(5);
      END_STATE();
    case 290:
      ACCEPT_TOKEN(sym_token);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(290);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [sym__ws] = ACTIONS(3),
    [sym_token] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(4),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(5),
    [sym__ws] = ACTIONS(3),
    [sym_token] = ACTIONS(7),
  },
  [2] = {
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(9),
    [sym__ws] = ACTIONS(3),
    [sym_token] = ACTIONS(11),
  },
  [3] = {
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(13),
    [sym__ws] = ACTIONS(3),
    [sym_token] = ACTIONS(15),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 2,
    ACTIONS(3), 1,
      sym__ws,
    ACTIONS(18), 1,
      ts_builtin_sym_end,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(4)] = 0,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [7] = {.entry = {.count = 1, .reusable = false}}, SHIFT(2),
  [9] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [11] = {.entry = {.count = 1, .reusable = false}}, SHIFT(3),
  [13] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [15] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(3),
  [18] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef TREE_SITTER_HIDE_SYMBOLS
#define TS_PUBLIC
#elif defined(_WIN32)
#define TS_PUBLIC __declspec(dllexport)
#else
#define TS_PUBLIC __attribute__((visibility("default")))
#endif

TS_PUBLIC const TSLanguage *tree_sitter_lale(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
