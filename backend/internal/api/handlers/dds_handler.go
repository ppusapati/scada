package handlers

import (
	"fmt"
	"net/http"
	"time"

	"scada-system/internal/dds"
)

// DDSHandler serves DDS status and discovery endpoints
type DDSHandler struct {
	subscriber *dds.Subscriber
	publisher  *dds.Publisher
	discovery  *dds.DiscoveryService
}

// NewDDSHandler creates a new handler for DDS-related API endpoints
func NewDDSHandler(subscriber *dds.Subscriber, publisher *dds.Publisher, discovery *dds.DiscoveryService) *DDSHandler {
	return &DDSHandler{
		subscriber: subscriber,
		publisher:  publisher,
		discovery:  discovery,
	}
}

// GetStatus returns the overall DDS domain status including participant count,
// topic information, QoS profiles, and subscriber/publisher metrics.
// GET /api/v1/dds/status
func (h *DDSHandler) GetStatus(w http.ResponseWriter, r *http.Request) {
	subMetrics := h.subscriber.GetMetrics()
	pubMetrics := h.publisher.GetMetrics()
	discMetrics := h.discovery.GetMetrics()

	// Build topic QoS info
	topicQoS := make(map[string]map[string]string)
	for _, topic := range dds.AllSubscribeTopics() {
		qos := dds.QoSForTopic(topic)
		topicQoS[string(topic)] = map[string]string{
			"reliability": qos.Reliability.Kind.String(),
			"durability":  qos.Durability.Kind.String(),
			"history":     fmt.Sprintf("%s (depth: %d)", qos.History.Kind.String(), qos.History.Depth),
		}
	}
	for _, topic := range dds.AllPublishTopics() {
		qos := dds.QoSForTopic(topic)
		topicQoS[string(topic)] = map[string]string{
			"reliability": qos.Reliability.Kind.String(),
			"durability":  qos.Durability.Kind.String(),
			"history":     fmt.Sprintf("%s (depth: %d)", qos.History.Kind.String(), qos.History.Depth),
		}
	}

	// Discovered topics from participants
	discoveredTopics := h.discovery.GetAllTopics()

	status := map[string]interface{}{
		"timestamp": time.Now().UTC(),
		"domain": map[string]interface{}{
			"active_participants": discMetrics.ActiveParticipants,
			"spdp_sent":          discMetrics.SPDPSent,
			"spdp_received":      discMetrics.SPDPReceived,
			"sedp_received":      discMetrics.SEDPReceived,
		},
		"subscriber": map[string]interface{}{
			"connected":          subMetrics.Connected,
			"running":            subMetrics.Running,
			"messages_received":  subMetrics.MessagesReceived,
			"messages_decoded":   subMetrics.MessagesDecoded,
			"decode_errors":      subMetrics.DecodeErrors,
			"messages_routed":    subMetrics.MessagesRouted,
			"fallback_to_mqtt":   subMetrics.FallbackToMQTT,
		},
		"publisher": map[string]interface{}{
			"running":       pubMetrics.Running,
			"messages_sent": pubMetrics.MessagesSent,
			"encode_errors": pubMetrics.EncodeErrors,
			"send_errors":   pubMetrics.SendErrors,
			"ack_received":  pubMetrics.AckReceived,
		},
		"subscribe_topics": dds.AllSubscribeTopics(),
		"publish_topics":   dds.AllPublishTopics(),
		"topic_qos":        topicQoS,
		"discovered_topics": discoveredTopics,
	}

	writeJSON(w, http.StatusOK, status)
}

// GetParticipants returns all discovered DDS domain participants with their
// endpoints and connection info.
// GET /api/v1/dds/participants
func (h *DDSHandler) GetParticipants(w http.ResponseWriter, r *http.Request) {
	participants := h.discovery.GetParticipants()

	result := make([]map[string]interface{}, 0, len(participants))
	for _, p := range participants {
		endpoints := make([]map[string]interface{}, 0, len(p.Endpoints))
		for _, ep := range p.Endpoints {
			kind := "subscriber"
			if ep.IsWriter {
				kind = "publisher"
			}
			endpoints = append(endpoints, map[string]interface{}{
				"topic_name": ep.TopicName,
				"type_name":  ep.TypeName,
				"kind":       kind,
				"qos": map[string]string{
					"reliability": ep.QoS.Reliability.Kind.String(),
					"durability":  ep.QoS.Durability.Kind.String(),
				},
			})
		}

		result = append(result, map[string]interface{}{
			"guid_prefix":       fmt.Sprintf("%x", p.GUIDPrefix),
			"participant_name":  p.ParticipantName,
			"protocol_version":  fmt.Sprintf("0x%04X", p.ProtocolVer),
			"vendor_id":         fmt.Sprintf("0x%04X", p.VendorID),
			"unicast_locator":   p.UnicastLocator,
			"multicast_locator": p.MulticastLocator,
			"lease_duration_sec": p.LeaseDuration.Seconds(),
			"discovered_at":     p.DiscoveredAt.UTC(),
			"last_seen":         p.LastSeen.UTC(),
			"stale":             p.IsStale(),
			"endpoints":         endpoints,
			"endpoint_count":    len(p.Endpoints),
		})
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"count":        len(result),
		"participants": result,
	})
}
