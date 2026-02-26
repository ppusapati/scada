package handler

import (
	"context"

	"connectrpc.com/connect"
	"google.golang.org/protobuf/types/known/timestamppb"

	"scada-system/domain/entity"
	scadav1 "scada-system/gen/scada/v1"
	"scada-system/gen/scada/v1/scadav1connect"
	solaruc "scada-system/usecase/solar"
)

type SolarServer struct {
	scadav1connect.UnimplementedSolarServiceHandler
	uc *solaruc.UseCase
}

func NewSolarServer(uc *solaruc.UseCase) *SolarServer {
	return &SolarServer{uc: uc}
}

func (s *SolarServer) GetSystems(ctx context.Context, req *connect.Request[scadav1.GetSolarSystemsRequest]) (*connect.Response[scadav1.GetSolarSystemsResponse], error) {
	systems, err := s.uc.GetAllSystems(ctx)
	if err != nil {
		return nil, toConnectError(err)
	}

	pbSystems := make([]*scadav1.SolarSystem, len(systems))
	for i, sys := range systems {
		pbSystems[i] = solarEntityToProto(&sys)
	}

	return connect.NewResponse(&scadav1.GetSolarSystemsResponse{
		Systems: pbSystems,
	}), nil
}

func (s *SolarServer) GetSystem(ctx context.Context, req *connect.Request[scadav1.GetSolarSystemRequest]) (*connect.Response[scadav1.SolarSystemResponse], error) {
	system, err := s.uc.GetSystem(ctx, req.Msg.SystemId)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.SolarSystemResponse{
		System: solarEntityToProto(system),
	}), nil
}

func (s *SolarServer) ControlInverter(ctx context.Context, req *connect.Request[scadav1.ControlInverterRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	userID := getUserIDFromCtx(ctx)
	if err := s.uc.ControlInverter(ctx, req.Msg.SystemId, req.Msg.Action, userID); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "inverter " + req.Msg.Action + " command sent",
	}), nil
}

func (s *SolarServer) SetMode(ctx context.Context, req *connect.Request[scadav1.SetSolarModeRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	mode := protoModeToEntity(req.Msg.Mode)
	if err := s.uc.SetOperatingMode(ctx, req.Msg.SystemId, mode); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "mode updated",
	}), nil
}

func (s *SolarServer) GetDevices(ctx context.Context, req *connect.Request[scadav1.GetDevicesRequest]) (*connect.Response[scadav1.GetDevicesResponse], error) {
	devices, err := s.uc.GetDevices(ctx)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.GetDevicesResponse{
		Devices: devicesToProto(devices),
	}), nil
}

func (s *SolarServer) GetDailyProduction(ctx context.Context, req *connect.Request[scadav1.GetDailyProductionRequest]) (*connect.Response[scadav1.DailyProductionResponse], error) {
	prod, err := s.uc.GetDailyProduction(ctx, req.Msg.SystemId)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.DailyProductionResponse{
		SystemId:        req.Msg.SystemId,
		DailyEnergyKwh: prod["daily_energy_kwh"].(float64),
		TotalEnergyMwh: prod["total_energy_mwh"].(float64),
		CurrentOutputKw: prod["current_output_kw"].(float64),
		Efficiency:      prod["efficiency"].(float64),
	}), nil
}

func solarEntityToProto(s *entity.SolarSystem) *scadav1.SolarSystem {
	return &scadav1.SolarSystem{
		Id:               s.ID,
		Name:             s.Name,
		TotalCapacityKw:  s.TotalCapacityKW,
		CurrentOutputKw:  s.CurrentOutputKW,
		Irradiance:       s.Irradiance,
		PanelTemperature: s.PanelTemperature,
		InverterStatus:   s.InverterStatus,
		GridVoltage:      s.GridVoltage,
		GridFrequency:    s.GridFrequency,
		DailyEnergyKwh:  s.DailyEnergyKWH,
		TotalEnergyMwh:  s.TotalEnergyMWH,
		Efficiency:       s.Efficiency,
		OperatingMode:    entityModeToProto(s.OperatingMode),
		UpdatedAt:        timestamppb.New(s.UpdatedAt),
	}
}
