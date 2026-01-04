// reaper_toneforge.cpp
// ToneForge REAPER Extension - HTTP API Bridge

#define WIN32_LEAN_AND_MEAN  // Winsock çakışmasını önle
#include <windows.h>

// WDL types tanımlamaları (REAPER SDK dependency)
#ifndef WDL_INT64
  #ifdef _MSC_VER
    #define WDL_INT64 __int64
  #else
    #define WDL_INT64 long long
  #endif
#endif

#define REAPERAPI_MINIMAL  // Sadece gerekli fonksiyonları al
#define REAPERAPI_IMPLEMENT
#define REAPERAPI_FUNCNAME(x) p_##x  // REAPER fonksiyonlar??n?? p_ prefix'iyle kullan
#include "reaper_plugin.h"  // Önce reaper_plugin.h

// İhtiyacımız olan fonksiyonları seç
#define REAPERAPI_WANT_CountTracks
#define REAPERAPI_WANT_GetTrack
#define REAPERAPI_WANT_GetTrackName
#define REAPERAPI_WANT_TrackFX_GetCount
#define REAPERAPI_WANT_TrackFX_GetFXName
#define REAPERAPI_WANT_TrackFX_GetNumParams
#define REAPERAPI_WANT_TrackFX_GetParamName
#define REAPERAPI_WANT_TrackFX_SetParamNormalized
#define REAPERAPI_WANT_TrackFX_GetParamNormalized
#define REAPERAPI_WANT_TrackFX_GetFormattedParamValue
#define REAPERAPI_WANT_TrackFX_AddByName
#define REAPERAPI_WANT_TrackFX_Delete
#define REAPERAPI_WANT_EnumInstalledFX
#define REAPERAPI_WANT_InsertTrackAtIndex
#define REAPERAPI_WANT_DeleteTrack
#define REAPERAPI_WANT_TrackFX_GetEnabled
#define REAPERAPI_WANT_TrackFX_SetEnabled
#define REAPERAPI_WANT_SetCurrentBPM
#define REAPERAPI_WANT_GetProjectTimeSignature2
#define REAPERAPI_WANT_Main_SaveProject
#define REAPERAPI_WANT_Main_openProject
#define REAPERAPI_WANT_GetProjectPath
#define REAPERAPI_WANT_CSurf_OnPlayRateChange

#include "reaper_plugin_functions.h"  // Sonra functions

// REAPER SDK typedef fixes (case sensitivity issues)
typedef ReaProject Reaproject;
#include <thread>
#include <string>
#include <map>
#include <vector>
#include <algorithm>
#include <sstream>
#include <mutex>
#include <cstdio>
#include <cctype>

// Minimal HTTP Server (single-header için)
// CPPHTTPLIB_OPENSSL_SUPPORT - OpenSSL kullanmıyoruz (localhost HTTP)
#include "httplib.h"

// JSON parser (single-header)
#include <nlohmann/json.hpp>
using json = nlohmann::json;

// Global state
static REAPER_PLUGIN_HINSTANCE g_hInstance = nullptr;
static httplib::Server g_server;
static std::thread g_server_thread;
static std::mutex g_api_mutex;
static std::map<std::string, json> g_plugin_cache;

struct ParamMetadata {
    std::string display;
    std::string unit;
    std::string format_hint;
};

ParamMetadata GetParamMetadata(MediaTrack* track, int fx_idx, int param_idx) {
    ParamMetadata meta{"", "", "raw"};

    if (!track || !p_TrackFX_GetFormattedParamValue) {
        return meta;
    }

    char formatted[256] = {0};
    if (p_TrackFX_GetFormattedParamValue(track, fx_idx, param_idx, formatted, sizeof(formatted))) {
        meta.display = formatted;
    }

    std::string lower = meta.display;
    std::transform(lower.begin(), lower.end(), lower.begin(), ::tolower);

    auto contains = [&lower](const std::string& needle) {
        return lower.find(needle) != std::string::npos;
    };

    if (contains("db")) {
        meta.unit = "dB";
        meta.format_hint = "decibel";
    } else if (contains("khz") || contains("hz")) {
        meta.unit = "Hz";
        meta.format_hint = "frequency";
    } else if (contains("%")) {
        meta.unit = "%";
        meta.format_hint = "percentage";
    } else if (contains("ms")) {
        meta.unit = "ms";
        meta.format_hint = "time";
    } else if (
        contains(" sec") ||
        contains("sec") ||
        (lower.size() > 1 &&
         lower.back() == 's' &&
         std::isdigit(static_cast<unsigned char>(lower[lower.size() - 2])))
    ) {
        meta.unit = "s";
        meta.format_hint = "time";
    }

    return meta;
}

// REAPER API function pointers - REAPERAPI_WANT ile otomatik tanımlanacak
// p_GetTrack, p_TrackFX_GetCount, vb. REAPER SDK tarafından sağlanır

// Helper: Get FX parameters as map (name -> index)
std::map<std::string, int> GetFXParamMap(MediaTrack* track, int fx_idx) {
    std::map<std::string, int> params;
    
    if (!track || !p_TrackFX_GetNumParams) return params;
    
    int param_count = p_TrackFX_GetNumParams(track, fx_idx);
    
    for (int i = 0; i < param_count; i++) {
        char param_name[256] = {0};
        if (p_TrackFX_GetParamName(track, fx_idx, i, param_name, 256)) {
            std::string name(param_name);
            // Normalize: lowercase, trim spaces
            std::transform(name.begin(), name.end(), name.begin(), ::tolower);
            
            // Remove spaces and special chars
            name.erase(std::remove_if(name.begin(), name.end(), 
                [](char c) { return !std::isalnum(static_cast<unsigned char>(c)); }), name.end());
            
            params[name] = i;
        }
    }
    
    return params;
}

std::string NormalizeParamName(const std::string& name) {
    std::string normalized = name;
    std::transform(normalized.begin(), normalized.end(), normalized.begin(), ::tolower);
    normalized.erase(std::remove_if(normalized.begin(), normalized.end(),
        [](char c) { return !std::isalnum(static_cast<unsigned char>(c)); }), normalized.end());
    return normalized;
}

json CollectFXParameters(MediaTrack* track, int fx_idx) {
    json params = json::array();

    if (!track || !p_TrackFX_GetNumParams) {
        return params;
    }

    int param_count = p_TrackFX_GetNumParams(track, fx_idx);
    for (int i = 0; i < param_count; ++i) {
        char param_name[256] = {0};
        if (p_TrackFX_GetParamName(track, fx_idx, i, param_name, 256)) {
            double default_value = p_TrackFX_GetParamNormalized(track, fx_idx, i);
            std::string raw_name(param_name);
            params.push_back({
                {"index", i},
                {"name_raw", raw_name},
                {"name_normalized", NormalizeParamName(raw_name)},
                {"default_normalized", default_value}
            });
        }
    }

    return params;
}

std::vector<std::string> EnumerateInstalledFX() {
    std::vector<std::string> fx_names;
    if (!p_EnumInstalledFX) {
        return fx_names;
    }

    const char* name_ptr = nullptr;
    const char* ident_ptr = nullptr;
    for (int i = 0; p_EnumInstalledFX(i, &name_ptr, &ident_ptr); ++i) {
        if (name_ptr) {
            fx_names.emplace_back(name_ptr);
        }
    }

    return fx_names;
}

MediaTrack* CreateTemporaryTrack(int& created_index, bool& created) {
    created = false;
    created_index = 0;

    if (p_CountTracks && p_InsertTrackAtIndex) {
        created_index = p_CountTracks(nullptr);
        p_InsertTrackAtIndex(created_index, true);
        created = true;
        return p_GetTrack(nullptr, created_index);
    }

    // Fallback: use first track if available
    return p_GetTrack ? p_GetTrack(nullptr, 0) : nullptr;
}

void CleanupTemporaryTrack(MediaTrack* track, int track_index, bool created) {
    if (created && p_DeleteTrack && track) {
        p_DeleteTrack(track);
    } else if (track && track_index >= 0) {
        p_TrackFX_Delete(track, track_index);
    }
}

json DescribePluginWithParams(const std::string& plugin_name) {
    int created_index = 0;
    bool created = false;
    MediaTrack* scan_track = CreateTemporaryTrack(created_index, created);

    if (!scan_track) {
        return json{{"error", "No track available for scanning"}, {"plugin", plugin_name}};
    }

    int fx_index = p_TrackFX_AddByName(scan_track, plugin_name.c_str(), false, -1);
    if (fx_index < 0) {
        CleanupTemporaryTrack(scan_track, fx_index, created);
        return json{{"error", "Failed to instantiate plugin"}, {"plugin", plugin_name}};
    }

    json params = CollectFXParameters(scan_track, fx_index);

    CleanupTemporaryTrack(scan_track, fx_index, created);

    std::string format = "unknown";
    auto colon_pos = plugin_name.find(":");
    if (colon_pos != std::string::npos) {
        format = plugin_name.substr(0, colon_pos);
    }

    return json{
        {"name", plugin_name},
        {"format", format},
        {"param_count", params.size()},
        {"params", params}
    };
}

// Fuzzy parameter search
int FindParamIndex(const std::map<std::string, int>& params, const std::string& search_term) {
    std::string search_lower = search_term;
    std::transform(search_lower.begin(), search_lower.end(), search_lower.begin(), ::tolower);
    search_lower.erase(std::remove_if(search_lower.begin(), search_lower.end(), 
        [](char c) { return !std::isalnum(static_cast<unsigned char>(c)); }), search_lower.end());
    
    // Exact match first
    auto it = params.find(search_lower);
    if (it != params.end()) {
        return it->second;
    }
    
    // Partial match (substring)
    for (const auto& [name, idx] : params) {
        if (name.find(search_lower) != std::string::npos) {
            return idx;
        }
    }
    
    return -1; // Not found
}

// HTTP Endpoint Handlers
void SetupHTTPEndpoints() {
    
    // Health check
    g_server.Get("/ping", [](const httplib::Request&, httplib::Response& res) {
        res.set_content(R"({"status":"ok","service":"ToneForge REAPER Extension"})", "application/json");
    });
    
    // Get track FX list
    g_server.Get("/fx/list", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        int track_idx = 0;
        if (req.has_param("track")) {
            track_idx = std::stoi(req.get_param_value("track"));
        }
        
        MediaTrack* track = p_GetTrack(nullptr, track_idx);
        if (!track) {
            res.status = 404;
            res.set_content(R"({"error":"Track not found"})", "application/json");
            return;
        }
        
        json fx_list = json::array();
        int fx_count = p_TrackFX_GetCount(track);
        
        for (int i = 0; i < fx_count; i++) {
            char fx_name[256] = {0};
            p_TrackFX_GetFXName(track, i, fx_name, 256);
            
            fx_list.push_back({
                {"index", i},
                {"name", fx_name}
            });
        }
        
        json response = {
            {"track", track_idx},
            {"fx_count", fx_count},
            {"fx_list", fx_list}
        };
        
        res.set_content(response.dump(), "application/json");
    });

    // Enumerate installed FX plugins and their default parameter states
    g_server.Get("/fx/catalog", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);

        bool refresh = req.has_param("refresh") && req.get_param_value("refresh") != "0";
        if (refresh) {
            g_plugin_cache.clear();
        }

        json plugins = json::array();
        auto installed_fx = EnumerateInstalledFX();

        for (const auto& fx_name : installed_fx) {
            if (!refresh) {
                auto cached = g_plugin_cache.find(fx_name);
                if (cached != g_plugin_cache.end()) {
                    plugins.push_back(cached->second);
                    continue;
                }
            }

            json plugin_info = DescribePluginWithParams(fx_name);
            g_plugin_cache[fx_name] = plugin_info;
            plugins.push_back(plugin_info);
        }

        json response = {
            {"count", plugins.size()},
            {"plugins", plugins},
            {"cache_size", g_plugin_cache.size()},
            {"refreshed", refresh}
        };

        res.set_content(response.dump(), "application/json");
    });
    
    // Set FX parameter
    g_server.Post("/fx/param", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            json body = json::parse(req.body);
            
            int track_idx = body.value("track", 0);
            int fx_idx = body.value("fx", 0);
            std::string param_name = body.value("param", "");
            double value = body.value("value", 0.0);
            
            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }
            
            // Get param map and find index
            auto params = GetFXParamMap(track, fx_idx);
            int param_idx = FindParamIndex(params, param_name);
            
            if (param_idx < 0) {
                res.status = 404;
                json error_response = {
                    {"error", "Parameter not found"},
                    {"searched", param_name},
                    {"available_params", json::array()}
                };
                
                for (const auto& [name, idx] : params) {
                    error_response["available_params"].push_back(name);
                }
                
                res.set_content(error_response.dump(), "application/json");
                return;
            }
            
            // Set parameter
            p_TrackFX_SetParamNormalized(track, fx_idx, param_idx, value);
            
            json response = {
                {"success", true},
                {"track", track_idx},
                {"fx", fx_idx},
                {"param_index", param_idx},
                {"value", value}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });

    // Set FX parameter by index (no fuzzy name matching)
    g_server.Post("/fx/param_index", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);

        try {
            json body = json::parse(req.body);

            int track_idx = body.value("track", 0);
            int fx_idx = body.value("fx", 0);
            int param_idx = body.value("param_index", -1);
            double value = body.value("value", 0.0);

            if (param_idx < 0) {
                res.status = 400;
                res.set_content(R"({"error":"param_index is required"})", "application/json");
                return;
            }

            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }

            int fx_count = p_TrackFX_GetCount(track);
            if (fx_idx < 0 || fx_idx >= fx_count) {
                res.status = 404;
                res.set_content(R"({"error":"FX not found"})", "application/json");
                return;
            }

            int param_count = p_TrackFX_GetNumParams(track, fx_idx);
            if (param_idx < 0 || param_idx >= param_count) {
                res.status = 404;
                res.set_content(R"({"error":"Parameter index out of range"})", "application/json");
                return;
            }

            p_TrackFX_SetParamNormalized(track, fx_idx, param_idx, value);

            char param_name[256] = {0};
            if (p_TrackFX_GetParamName) {
                p_TrackFX_GetParamName(track, fx_idx, param_idx, param_name, 256);
            }

            json response = {
                {"success", true},
                {"track", track_idx},
                {"fx", fx_idx},
                {"param_index", param_idx},
                {"param_name", std::string(param_name)},
                {"value", value}
            };

            res.set_content(response.dump(), "application/json");
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });

    // Get FX parameter by index (no fuzzy name matching)
    g_server.Get("/fx/param_index", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);

        try {
            int track_idx = std::stoi(req.get_param_value("track"));
            int fx_idx = std::stoi(req.get_param_value("fx"));
            int param_idx = std::stoi(req.get_param_value("param_index"));

            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }

            int fx_count = p_TrackFX_GetCount(track);
            if (fx_idx < 0 || fx_idx >= fx_count) {
                res.status = 404;
                res.set_content(R"({"error":"FX not found"})", "application/json");
                return;
            }

            int param_count = p_TrackFX_GetNumParams(track, fx_idx);
            if (param_idx < 0 || param_idx >= param_count) {
                res.status = 404;
                res.set_content(R"({"error":"Parameter index out of range"})", "application/json");
                return;
            }

            double value = p_TrackFX_GetParamNormalized(track, fx_idx, param_idx);

            char param_name[256] = {0};
            if (p_TrackFX_GetParamName) {
                p_TrackFX_GetParamName(track, fx_idx, param_idx, param_name, 256);
            }

            json response = {
                {"track", track_idx},
                {"fx", fx_idx},
                {"param_index", param_idx},
                {"param_name", std::string(param_name)},
                {"value", value}
            };

            res.set_content(response.dump(), "application/json");
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    // Get FX parameter value
    g_server.Get("/fx/param", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            int track_idx = std::stoi(req.get_param_value("track"));
            int fx_idx = std::stoi(req.get_param_value("fx"));
            std::string param_name = req.get_param_value("param");
            
            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }
            
            auto params = GetFXParamMap(track, fx_idx);
            int param_idx = FindParamIndex(params, param_name);
            
            if (param_idx < 0) {
                res.status = 404;
                res.set_content(R"({"error":"Parameter not found"})", "application/json");
                return;
            }
            
            double value = p_TrackFX_GetParamNormalized(track, fx_idx, param_idx);
            
            json response = {
                {"track", track_idx},
                {"fx", fx_idx},
                {"param", param_name},
                {"param_index", param_idx},
                {"value", value}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    // Add FX to track
    g_server.Post("/fx/add", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            json body = json::parse(req.body);
            
            int track_idx = body.value("track", 0);
            std::string plugin_name = body.value("plugin", "");
            
            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }
            
            // Add plugin (-1 means append to end)
            int new_fx_idx = p_TrackFX_AddByName(track, plugin_name.c_str(), false, -1);
            
            if (new_fx_idx < 0) {
                res.status = 500;
                json error = {
                    {"error", "Failed to load plugin"},
                    {"plugin", plugin_name}
                };
                res.set_content(error.dump(), "application/json");
                return;
            }
            
            char fx_name[256] = {0};
            p_TrackFX_GetFXName(track, new_fx_idx, fx_name, 256);
            
            json response = {
                {"success", true},
                {"track", track_idx},
                {"fx_index", new_fx_idx},
                {"fx_name", fx_name}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    // Remove FX from track
    g_server.Delete("/fx/remove", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            int track_idx = std::stoi(req.get_param_value("track"));
            int fx_idx = std::stoi(req.get_param_value("fx"));
            
            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }
            
            bool success = p_TrackFX_Delete(track, fx_idx);
            
            json response = {
                {"success", success},
                {"track", track_idx},
                {"fx", fx_idx}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    // Get all FX parameters for inspection
    g_server.Get("/fx/params", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            int track_idx = req.has_param("track") ? std::stoi(req.get_param_value("track")) : 0;
            int fx_idx = req.has_param("fx") ? std::stoi(req.get_param_value("fx")) : 0;
            
            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }
            
            int param_count = p_TrackFX_GetNumParams(track, fx_idx);
            json params = json::array();
            for (int i = 0; i < param_count; ++i) {
                char param_name[256] = {0};
                if (p_TrackFX_GetParamName(track, fx_idx, i, param_name, 256)) {
                    double value = p_TrackFX_GetParamNormalized(track, fx_idx, i);
                    ParamMetadata meta = GetParamMetadata(track, fx_idx, i);
                    params.push_back({
                        {"index", i},
                        {"name", std::string(param_name)},
                        {"value", value},
                        {"display", meta.display},
                        {"unit", meta.unit},
                        {"format_hint", meta.format_hint}
                    });
                }
            }
            
            json response = {
                {"track", track_idx},
                {"fx", fx_idx},
                {"params", params}
            };
            res.set_content(response.dump(), "application/json");
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });

    // Toggle FX enable/bypass state
    g_server.Post("/fx/toggle", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);

        try {
            json body = json::parse(req.body);

            int track_idx = body.value("track", 0);
            int fx_idx = body.value("fx", 0);
            bool enabled = body.value("enabled", true);

            MediaTrack* track = p_GetTrack(nullptr, track_idx);
            if (!track) {
                res.status = 404;
                res.set_content(R"({"error":"Track not found"})", "application/json");
                return;
            }

            p_TrackFX_SetEnabled(track, fx_idx, enabled);
            bool current_state = p_TrackFX_GetEnabled ? p_TrackFX_GetEnabled(track, fx_idx) : enabled;

            json response = {
                {"success", true},
                {"track", track_idx},
                {"fx", fx_idx},
                {"enabled", current_state}
            };

            res.set_content(response.dump(), "application/json");

        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    // BPM control
    g_server.Post("/transport/bpm", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            json body = json::parse(req.body);
            double bpm = body.value("bpm", 120.0);
            
            p_SetCurrentBPM(nullptr, bpm, true);
            
            json response = {
                {"success", true},
                {"bpm", bpm}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    g_server.Get("/transport/bpm", [](const httplib::Request&, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        double bpm = 0.0;
        double bpi = 0.0;  // beats per interval
        p_GetProjectTimeSignature2(nullptr, &bpm, &bpi);
        
        json response = {
            {"bpm", bpm},
            {"beats_per_measure", bpi}
        };
        
        res.set_content(response.dump(), "application/json");
    });

    // Get overview of all tracks and FX states
    g_server.Get("/tracks", [](const httplib::Request&, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);

        int track_count = p_CountTracks ? p_CountTracks(nullptr) : 0;
        json tracks = json::array();

        for (int t = 0; t < track_count; ++t) {
            MediaTrack* track = p_GetTrack(nullptr, t);
            if (!track) continue;

            char track_name[256] = {0};
            if (!p_GetTrackName(track, track_name, 256) || track_name[0] == '\0') {
                snprintf(track_name, sizeof(track_name), "Track %d", t + 1);
            }

            json fx_list = json::array();
            int fx_count = p_TrackFX_GetCount(track);
            for (int fx = 0; fx < fx_count; ++fx) {
                char fx_name[256] = {0};
                p_TrackFX_GetFXName(track, fx, fx_name, 256);
                bool enabled = p_TrackFX_GetEnabled ? p_TrackFX_GetEnabled(track, fx) : true;

                fx_list.push_back({
                    {"index", fx},
                    {"name", fx_name},
                    {"enabled", enabled}
                });
            }

            tracks.push_back({
                {"index", t},
                {"name", std::string(track_name)},
                {"fx_count", fx_count},
                {"fx_list", fx_list}
            });
        }

        json response = {
            {"track_count", track_count},
            {"tracks", tracks}
        };

        res.set_content(response.dump(), "application/json");
    });
    
    // Project management
    g_server.Post("/project/save", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            json body = json::parse(req.body);
            std::string preset_name = body.value("name", "preset");
            
            // Save current project
            p_Main_SaveProject(0, false);
            
            char project_path[512] = {0};
            p_GetProjectPath(project_path, 512);
            
            json response = {
                {"success", true},
                {"preset_name", preset_name},
                {"project_path", project_path}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
    
    g_server.Post("/project/load", [](const httplib::Request& req, httplib::Response& res) {
        std::lock_guard<std::mutex> lock(g_api_mutex);
        
        try {
            json body = json::parse(req.body);
            std::string project_path = body.value("path", "");
            
            if (project_path.empty()) {
                res.status = 400;
                res.set_content(R"({"error":"Project path required"})", "application/json");
                return;
            }
            
            p_Main_openProject(project_path.c_str());
            
            json response = {
                {"success", true},
                {"loaded_path", project_path}
            };
            
            res.set_content(response.dump(), "application/json");
            
        } catch (const std::exception& e) {
            res.status = 400;
            json error = {{"error", e.what()}};
            res.set_content(error.dump(), "application/json");
        }
    });
}

void StartHTTPServer() {
    SetupHTTPEndpoints();
    
    g_server_thread = std::thread([]() {
        // Start server on localhost:8888
        g_server.listen("127.0.0.1", 8888);
    });
}

void StopHTTPServer() {
    g_server.stop();
    if (g_server_thread.joinable()) {
        g_server_thread.join();
    }
}

// REAPER Extension Entry Point
extern "C" REAPER_PLUGIN_DLL_EXPORT int REAPER_PLUGIN_ENTRYPOINT(
    REAPER_PLUGIN_HINSTANCE hInstance,
    reaper_plugin_info_t *rec
) {
    if (!rec) {
        // Unloading
        StopHTTPServer();
        return 0;
    }
    
    if (rec->caller_version != REAPER_PLUGIN_VERSION || !rec->GetFunc) {
        return 0;
    }
    
    g_hInstance = hInstance;
    
    // Import REAPER API functions
    #define IMPAPI(x) p_##x = (decltype(p_##x))rec->GetFunc(#x); if (!p_##x) return 0
    IMPAPI(CountTracks);
    IMPAPI(GetTrack);
    IMPAPI(GetTrackName);
    IMPAPI(TrackFX_GetCount);
    IMPAPI(TrackFX_GetFXName);
    IMPAPI(TrackFX_GetNumParams);
    IMPAPI(TrackFX_GetParamName);
    IMPAPI(TrackFX_SetParamNormalized);
    IMPAPI(TrackFX_GetParamNormalized);
    IMPAPI(TrackFX_GetFormattedParamValue);
    IMPAPI(TrackFX_AddByName);
    IMPAPI(TrackFX_Delete);
    IMPAPI(EnumInstalledFX);
    IMPAPI(InsertTrackAtIndex);
    IMPAPI(DeleteTrack);
    IMPAPI(TrackFX_GetEnabled);
    IMPAPI(TrackFX_SetEnabled);
    IMPAPI(SetCurrentBPM);
    IMPAPI(GetProjectTimeSignature2);
    IMPAPI(Main_SaveProject);
    IMPAPI(Main_openProject);
    IMPAPI(GetProjectPath);
    #undef IMPAPI
    
    // Start HTTP server
    StartHTTPServer();
    
    return 1; // Success
}
