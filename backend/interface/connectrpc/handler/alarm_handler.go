package handler

import (
	"context"
	"time"

	"connectrpc.com/connect"
	"google.golang.org/protobuf/types/known/timestamppb"

	"scada-system/domain/entity"
	"scada-system/domain/valueobject"
	scadav1 "scada-system/gen/scada/v1"
	"scada-system/gen/scada/v1/scadav1connect"
	alarmuc "scada-system/usecase/alarm"
)

type AlarmServer struct {
	scadav1connect.UnimplementedAlarmServiceHandler
	uc *alarmuc.UseCase
}

func NewAlarmServer(uc *alarmuc.UseCase) *AlarmServer {
	return &AlarmServer{uc: uc}
}

func (s *AlarmServer) GetActiveAlarms(ctx context.Context, req *connect.Request[scadav1.GetActiveAlarmsRequest]) (*connect.Response[scadav1.GetAlarmsResponse], error) {
	alarms, err := s.uc.GetActive(ctx, req.Msg.Subsystem)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.GetAlarmsResponse{
		Alarms: alarmsToProto(alarms),
	}), nil
}

func (s *AlarmServer) GetAlarmHistory(ctx context.Context, req *connect.Request[scadav1.GetAlarmHistoryRequest]) (*connect.Response[scadav1.GetAlarmsResponse], error) {
	tr := valueobject.Last24Hours()
	if req.Msg.TimeRange != nil {
		tr = valueobject.NewTimeRange(
			req.Msg.TimeRange.From.AsTime(),
			req.Msg.TimeRange.To.AsTime(),
		)
	}
	limit := int(req.Msg.Limit)
	if limit <= 0 {
		limit = 500
	}

	alarms, err := s.uc.GetHistory(ctx, req.Msg.Subsystem, tr, limit)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.GetAlarmsResponse{
		Alarms: alarmsToProto(alarms),
	}), nil
}

func (s *AlarmServer) AcknowledgeAlarm(ctx context.Context, req *connect.Request[scadav1.AcknowledgeAlarmRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	userID := getUserIDFromCtx(ctx)
	if err := s.uc.Acknowledge(ctx, req.Msg.AlarmId, userID); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "alarm acknowledged",
	}), nil
}

func (s *AlarmServer) ClearAlarm(ctx context.Context, req *connect.Request[scadav1.ClearAlarmRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	if err := s.uc.Clear(ctx, req.Msg.AlarmId); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "alarm cleared",
	}), nil
}

func (s *AlarmServer) GetAlarmCounts(ctx context.Context, req *connect.Request[scadav1.GetAlarmCountsRequest]) (*connect.Response[scadav1.AlarmCountsResponse], error) {
	counts, err := s.uc.GetCounts(ctx)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.AlarmCountsResponse{
		Critical: int32(counts.Critical),
		Warning:  int32(counts.Warning),
		Info:     int32(counts.Info),
	}), nil
}

func alarmsToProto(alarms []entity.Alarm) []*scadav1.Alarm {
	result := make([]*scadav1.Alarm, len(alarms))
	for i, a := range alarms {
		pa := &scadav1.Alarm{
			Id:         a.ID,
			DeviceId:   a.DeviceID,
			Subsystem:  string(a.Subsystem),
			Severity:   alarmSeverityToProto(a.Severity),
			Category:   string(a.Category),
			Message:    a.Message,
			Value:      a.Value,
			Threshold:  a.Threshold,
			Status:     string(a.Status),
			OccurredAt: timestamppb.New(a.OccurredAt),
			AckedBy:    a.AckedBy,
		}
		if a.AckedAt != nil {
			pa.AckedAt = timestamppb.New(*a.AckedAt)
		}
		if a.ClearedAt != nil {
			pa.ClearedAt = timestamppb.New(*a.ClearedAt)
		}
		result[i] = pa
	}
	return result
}

func alarmSeverityToProto(s entity.AlarmSeverity) scadav1.AlarmSeverity {
	switch s {
	case entity.SeverityCritical:
		return scadav1.AlarmSeverity_ALARM_SEVERITY_CRITICAL
	case entity.SeverityWarning:
		return scadav1.AlarmSeverity_ALARM_SEVERITY_WARNING
	case entity.SeverityInfo:
		return scadav1.AlarmSeverity_ALARM_SEVERITY_INFO
	default:
		return scadav1.AlarmSeverity_ALARM_SEVERITY_UNSPECIFIED
	}
}

// helper to avoid unused import
var _ = time.Now
